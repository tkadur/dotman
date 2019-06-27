mod rcrc;

use derive_getters::Getters;
use derive_more::From;
use gethostname::gethostname;
use globset::Glob;
use std::{collections::HashSet, error, fmt::{self, Display}, path::PathBuf};
use walkdir::WalkDir;

#[derive(Debug, From)]
pub enum Error {
    NoSystemHostname,
    NoHomeDirectory,
    WalkdirError(walkdir::Error),
    RcrcError(rcrc::Error),
}
use self::Error::*;

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let error_msg = match self {
            NoSystemHostname => String::from("reading system hostname"),
            NoHomeDirectory => String::from("finding home directory"),
            WalkdirError(error) => format!("reading file or directory ({})", error.to_string()),
            RcrcError(error) => error.to_string(),
        };

        write!(f, "error {}", error_msg)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            WalkdirError(error) => Some(error),
            RcrcError(error) => Some(error),
            NoSystemHostname | NoHomeDirectory => None,
        }
    }
}

/// All rcm configuration options
// `rcrc::Config` is more "raw" than this because it's meant to be a direct translation
// of the user's rcrc file. This type encompasses all possible configuration options,
// so it eliminates the optional-ness of `rcrc::Config`'s fields
#[derive(Debug, Getters)]
pub struct Config {
    verbose: bool,
    excludes: Vec<PathBuf>,
    tags: Vec<String>,
    dotfiles_path: PathBuf,
    hostname: String,
}

#[derive(Debug)]
enum PartialSource {
    Cli,
    Default,
}

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
        let verbose = cli.verbose.unwrap_or(default.config.verbose);
        let excludes = merge_vecs(
            cli.excludes.unwrap_or_else(|| vec![]),
            default.config.excludes,
        );
        let tags = merge_vecs(cli.tags.unwrap_or_else(|| vec![]), default.config.tags);

        fn merge_with_source<T>(cli: Option<T>, default: T) -> (T, PartialSource) {
            cli.map(|c| (c, PartialSource::Cli))
                .unwrap_or((default, PartialSource::Default))
        }
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
}

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

        fn values_to_vec<'a, T>(
            args: &'a clap::ArgMatches,
            name: &str,
            f: impl Fn(&'a str) -> T,
        ) -> Option<Vec<T>> {
            args.values_of(name).map(|e| e.map(f).collect())
        }
        let excludes = values_to_vec(args, "excludes", PathBuf::from);
        let tags = values_to_vec(args, "tags", String::from);

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
            .join("dotfiles");

        let hostname = gethostname()
            .to_str()
            .ok_or(NoSystemHostname)?
            .to_owned();

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

fn merge_vecs<T>(x: Vec<T>, mut y: Vec<T>) -> Vec<T> {
    let mut res = x;
    res.append(&mut y);

    res
}

fn merge_rcrc(
    partial_config: PartialConfig,
    rcrc_config: rcrc::Config,
) -> Result<Config, Error> {
    let verbose = partial_config.verbose;

    // Makes sure to respect the hierarchy of selecting in the following order
    // - CLI
    // - rcrc
    // - Default source
    fn merge_hierarchy<T>(partial: (T, PartialSource), rcrc: Option<T>) -> T {
        match partial {
            (x, PartialSource::Cli) => x,
            (x, PartialSource::Default) => rcrc.unwrap_or(x),
        }
    }
    let dotfiles_path = {
        fn expand_tilde(path: String) -> Option<String> {
            Some(
                if path.starts_with('~') {
                    path.replacen("~", dirs::home_dir()?.to_str()?, 1)
                } else {
                    path
                },
            )
        }

        let rcrc_dotfiles_path = rcrc_config
            .dotfiles_path
            .and_then(expand_tilde)
            .map(PathBuf::from);

        merge_hierarchy(partial_config.dotfiles_path, rcrc_dotfiles_path)
    };
    debug_assert!(dotfiles_path.is_absolute());

    let excludes = {
        let mut excludes: Vec<PathBuf> =
            // Merge the excludes from partial_config (CLI + default) with the excludes from the rcrc
            merge_vecs(
                partial_config.excludes,
                // We need to handle the possibility of the rcrc not specifying any excludes,
                // as well as converting from the raw String input to a PathBuf
                rcrc_config
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

        // Finally, remove any duplicate entries from files matching multiple globs
        let set: HashSet<_> = excludes.drain(..).collect();
        excludes.extend(set.into_iter());

        excludes
    };

    let tags = merge_vecs(
        partial_config.tags,
        rcrc_config.tags.unwrap_or_else(|| vec![]),
    );

    let hostname = merge_hierarchy(partial_config.hostname, rcrc_config.hostname);

    Ok(Config {
        verbose,
        excludes,
        tags,
        dotfiles_path,
        hostname,
    })
}

// TODO Finish
fn find_rcrc(_partial_config: &PartialConfig) -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".rcrc-test"))
}

pub fn get(cli_args: &clap::ArgMatches) -> Result<Config, Error> {
    let partial_config = PartialConfig::merge(CliConfig::get(cli_args), DefaultConfig::get()?);
    let rcrc_config = rcrc::get(find_rcrc(&partial_config))?;
    let config = merge_rcrc(partial_config, rcrc_config)?;

    Ok(config)
}
