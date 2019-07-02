mod dotrc;

use crate::common::util;
use derive_getters::Getters;
use derive_more::From;
use gethostname::gethostname;
use globset::Glob;
use std::{
    collections::HashSet,
    error,
    fmt::{self, Display},
    path::PathBuf,
};
use walkdir::WalkDir;

const DEFAULT_DOTFILES_DIR: &str = ".dotfiles";
const DOTRC_NAME: &str = ".dotrc.toml";

#[derive(Debug, From)]
pub enum Error {
    NoSystemHostname,
    NoHomeDirectory,
    WalkdirError(walkdir::Error),
    DotrcError(dotrc::Error),
}
use self::Error::*;

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let error_msg = match self {
            NoSystemHostname => String::from("error reading system hostname"),
            NoHomeDirectory => String::from("error finding home directory"),
            WalkdirError(error) => {
                format!("error reading file or directory ({})", error.to_string())
            },
            DotrcError(error) => error.to_string(),
        };

        write!(f, "{}", error_msg)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            WalkdirError(error) => Some(error),
            DotrcError(error) => Some(error),
            NoSystemHostname | NoHomeDirectory => None,
        }
    }
}

/// All dotman configuration options
#[derive(Debug, Getters)]
pub struct Config {
    verbose: bool,
    excludes: Vec<PathBuf>,
    tags: Vec<String>,
    dotfiles_path: PathBuf,
    hostname: String,
}

#[derive(Debug, Clone)]
enum PartialSource {
    Cli,
    Default,
}

/// Configuration options sans dotrc.
///
/// Can be used to guide dotrc discovery with `find_rcrc`.
#[derive(Debug)]
struct PartialConfig {
    verbose: bool,
    excludes: Vec<PathBuf>,
    tags: Vec<String>,
    dotfiles_path: (PathBuf, PartialSource),
    hostname: (String, PartialSource),
}

impl PartialConfig {
    fn merge(cli: CliConfig, default: DefaultConfig) -> Self {
        /// Combines a single CLI item and a single default item, annotating the
        /// result with its source
        fn merge_with_source<T>(cli: Option<T>, default: T) -> (T, PartialSource) {
            cli.map(|c| (c, PartialSource::Cli))
                .unwrap_or((default, PartialSource::Default))
        }

        let verbose = cli.verbose.unwrap_or(default.config.verbose);

        let excludes = util::append_vecs(
            cli.excludes.unwrap_or_else(|| vec![]),
            default.config.excludes,
        );
        let tags = util::append_vecs(cli.tags.unwrap_or_else(|| vec![]), default.config.tags);

        let dotfiles_path = merge_with_source(cli.dotfiles_path, default.config.dotfiles_path);
        let hostname = merge_with_source(cli.hostname, default.config.hostname);

        PartialConfig {
            verbose,
            excludes,
            tags,
            dotfiles_path,
            hostname,
        }
    }

    fn to_config(&self) -> Config {
        let verbose = self.verbose;
        let excludes = self.excludes.clone();
        let tags = self.tags.clone();
        let (dotfiles_path, _) = self.dotfiles_path.clone();
        let (hostname, _) = self.hostname.clone();

        Config {
            verbose,
            excludes,
            tags,
            dotfiles_path,
            hostname,
        }
    }
}

/// The portion of the configuration read from CLI arguments and
/// environment variables
#[derive(Debug)]
struct CliConfig {
    verbose: Option<bool>,
    excludes: Option<Vec<PathBuf>>,
    tags: Option<Vec<String>>,
    dotfiles_path: Option<PathBuf>,
    hostname: Option<String>,
}

impl CliConfig {
    /// Gets a partial configuration from CLI arguments.
    fn get(args: &clap::ArgMatches) -> Self {
        let verbose = Some(args.is_present("verbose"));

        let excludes = args
            .values_of("excludes")
            .map(|e| e.map(PathBuf::from).collect());
        let tags = args
            .values_of("tags")
            .map(|t| t.map(String::from).collect());

        let dotfiles_path = args.value_of("dotfiles_path").map(PathBuf::from);
        let hostname = args.value_of("hostname").map(String::from);

        CliConfig {
            verbose,
            excludes,
            tags,
            dotfiles_path,
            hostname,
        }
    }
}

struct DefaultConfig {
    config: Config,
}

impl DefaultConfig {
    /// Gets a partial configuration corresponding to the "default"
    /// values/sources of each configuration option.
    fn get() -> Result<DefaultConfig, Error> {
        let verbose = false;
        let excludes = vec![];
        let tags = vec![];

        let dotfiles_path = dirs::home_dir()
            .ok_or(NoHomeDirectory)?
            .join(DEFAULT_DOTFILES_DIR);

        let hostname = gethostname().to_str().ok_or(NoSystemHostname)?.to_owned();

        Ok(DefaultConfig {
            config: Config {
                verbose,
                excludes,
                tags,
                dotfiles_path,
                hostname,
            },
        })
    }
}

/// Merges a partial config (obtained from the CLI and default settings) with a
/// config obtained from reading the dotrc to create a complete configuration.
fn merge_dotrc(
    partial_config: PartialConfig,
    dotrc_config: dotrc::Config,
) -> Result<Config, Error> {
    let verbose = partial_config.verbose;

    /// Merges an item from a `PartialConfig` and an item from a
    /// `dotrc::Config`, making sure to respect the hierarchy of selecting
    /// in the following order
    /// - CLI
    /// - dotrc
    /// - Default source
    fn merge_hierarchy<T>(partial: (T, PartialSource), dotrc: Option<T>) -> T {
        match partial {
            (x, PartialSource::Cli) => x,
            (x, PartialSource::Default) => dotrc.unwrap_or(x),
        }
    }

    /// If `path` begins with a tilde, attempts to expand it into the
    /// full home directory path. If `path` doesn't start with a tilde, just
    /// successfully returns path. Fails if the home directory cannot
    /// be read as a `String`.
    fn expand_tilde(path: String) -> Option<String> {
        Some(
            if path.starts_with('~') {
                path.replacen("~", dirs::home_dir()?.to_str()?, 1)
            } else {
                path
            },
        )
    }

    let dotfiles_path = {
        let dotrc_dotfiles_path = dotrc_config
            .dotfiles_path
            .and_then(expand_tilde)
            .map(PathBuf::from);

        merge_hierarchy(partial_config.dotfiles_path, dotrc_dotfiles_path)
    };
    debug_assert!(dotfiles_path.is_absolute());

    let excludes = {
        let mut excludes: Vec<PathBuf> =
            // Merge the excludes from partial_config (CLI + default) with the excludes from the dotrc
            util::append_vecs(
                partial_config.excludes,
                // We need to handle the possibility of the dotrc not specifying any excludes,
                // as well as converting from the raw String input to a PathBuf
                dotrc_config
                    .excludes
                    .unwrap_or_else(|| vec![])
                    .iter()
                    .map(PathBuf::from)
                    .collect(),
            )
            .into_iter()
            // Try to glob expand each exclude
            // If PathBuf -> String conversion fails or the pattern is invalid,
            // fall back to simply not trying to glob-expand
            .map(|path: PathBuf| -> Result<Vec<PathBuf>, walkdir::Error> {
                let glob = match path.to_str().map(Glob::new) {
                    Some(Ok(glob)) => glob.compile_matcher(),
                    None | Some(Err(_)) => return Ok(vec![path]),
                };

                let entries: Vec<walkdir::DirEntry> = WalkDir::new(&dotfiles_path)
                    .follow_links(true)
                    .into_iter()
                    .collect::<Result<_, _>>()?;

                let paths = entries
                    .into_iter()
                    .filter_map(|entry| {
                        let entry_path = entry.path().strip_prefix(&dotfiles_path).expect("Entry must be in the dotfiles path");

                        if glob.is_match(entry_path) {
                            Some(PathBuf::from(entry_path))
                        } else {
                            None
                        }
                    })
                    .collect();

                Ok(paths)
            })
            // If any glob expansion failed due to an I/O error, give up
            .collect::<Result<Vec<Vec<PathBuf>>, _>>()?
            // Then flatten the glob-expanded results
            .into_iter()
            .flatten()
            .collect();

        // Finally, remove any duplicate entries due to files matching multiple globs
        let set: HashSet<_> = excludes.drain(..).collect();
        excludes.extend(set.into_iter());

        excludes
    };

    let tags = util::append_vecs(
        partial_config.tags,
        dotrc_config.tags.unwrap_or_else(|| vec![]),
    );

    let hostname = merge_hierarchy(partial_config.hostname, dotrc_config.hostname);

    Ok(Config {
        verbose,
        excludes,
        tags,
        dotfiles_path,
        hostname,
    })
}

/// Given the partial config built from CLI arguments and default values, tries
/// to find the dotrc file.
///
/// Searches the following locations, in order:
/// - The `host-` folder matching the hostname in `partial_config`
/// - Any `tag-` folders matching the tags in `partial_config` (the tags are
///   searched in an unspecified order)
/// - The default location (`~/.dotrc`)
fn find_dotrc(partial_config: &PartialConfig) -> Option<PathBuf> {
    let config = partial_config.to_config();

    let items = crate::resolver::get(&config).ok()?;
    for item in items {
        match item.dest().file_name() {
            Some(name) if name == DOTRC_NAME => return Some(item.source().clone()),
            _ => (),
        }
    }

    dirs::home_dir().map(|home| home.join(DOTRC_NAME))
}

pub fn get(cli_args: &clap::ArgMatches) -> Result<Config, Error> {
    let partial_config = PartialConfig::merge(CliConfig::get(cli_args), DefaultConfig::get()?);
    let dotrc_config = dotrc::get(find_dotrc(&partial_config))?;
    let config = merge_dotrc(partial_config, dotrc_config)?;

    Ok(config)
}
