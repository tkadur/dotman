pub mod cli;
mod dotrc;

use crate::{
    common::{util, AbsolutePath},
    verbose_println,
};
use derive_getters::Getters;
use derive_more::From;
use failure::Fail;
use gethostname::gethostname;
use globset::Glob;
use lazy_static::lazy_static;
use std::{
    collections::HashSet,
    ffi::OsStr,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

const DEFAULT_DOTFILES_DIR: &str = ".dotfiles";
lazy_static! {
    static ref DOTRC_NAMES: [&'static OsStr; 3] = [
        OsStr::new(".dotrc"),
        OsStr::new(".dotrc.yml"),
        OsStr::new(".dotrc.yaml")
    ];
}

/// All dotman configuration options
#[derive(Debug, Getters)]
pub struct Config {
    excludes: Vec<AbsolutePath>,
    tags: Vec<String>,
    dotfiles_path: AbsolutePath,
    hostname: String,
    command: cli::Command,
}

#[derive(Debug)]
enum PartialSource {
    Cli,
    Default,
}

/// Configuration options sans dotrc.
///
/// Can be used to guide dotrc discovery with `find_rcrc`.
#[derive(Debug)]
struct PartialConfig {
    excludes: Vec<PathBuf>,
    tags: Vec<String>,
    dotfiles_path: (PathBuf, PartialSource),
    hostname: (String, PartialSource),
    command: cli::Command,
}

impl PartialConfig {
    fn merge(cli: cli::Config, default: DefaultConfig) -> Self {
        let excludes = util::append_vecs(cli.excludes, default.excludes);

        let tags = util::append_vecs(cli.tags, default.tags);

        let dotfiles_path = match cli.dotfiles_path {
            Some(dotfiles_path) => (dotfiles_path, PartialSource::Cli),
            None => (default.dotfiles_path, PartialSource::Default),
        };

        let hostname = match cli.hostname {
            Some(hostname) => (hostname, PartialSource::Cli),
            None => (default.hostname, PartialSource::Default),
        };

        let command = cli.command;

        PartialConfig {
            excludes,
            tags,
            dotfiles_path,
            hostname,
            command,
        }
    }

    fn to_config(&self) -> Result<Config, walkdir::Error> {
        let dotfiles_path = {
            let (dotfiles_path, _) = &self.dotfiles_path;

            AbsolutePath::from(dotfiles_path.clone())
        };

        let excludes = self
            .excludes
            .iter()
            // Glob-expand
            .map(|exclude| expand_glob(exclude, &dotfiles_path))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            // Wrap with `AbsolutePath`s
            .map(|exclude| AbsolutePath::from(exclude.clone()))
            .collect();

        let tags = self.tags.clone();

        let hostname = {
            let (hostname, _) = &self.hostname;

            hostname.clone()
        };

        let command = self.command.clone();

        Ok(Config {
            excludes,
            tags,
            dotfiles_path,
            hostname,
            command,
        })
    }
}

struct DefaultConfig {
    excludes: Vec<PathBuf>,
    tags: Vec<String>,
    dotfiles_path: PathBuf,
    hostname: String,
}

impl DefaultConfig {
    /// Gets a partial configuration corresponding to the "default"
    /// values/sources of each configuration option.
    fn get() -> Result<DefaultConfig, Error> {
        let excludes = vec![];
        let tags = vec![];

        let dotfiles_path = util::home_dir().join(DEFAULT_DOTFILES_DIR);

        let hostname = gethostname().to_str().ok_or(NoSystemHostname)?.to_owned();

        Ok(DefaultConfig {
            excludes,
            tags,
            dotfiles_path,
            hostname,
        })
    }
}

/// Tries to glob-expand `path`.
/// If `PathBuf` -> `String` conversion fails or the pattern is invalid,
/// fall back to simply not trying to glob-expand
fn expand_glob(path: &Path, dotfiles_path: &AbsolutePath) -> Result<Vec<PathBuf>, walkdir::Error> {
    // Just to improve whitespace in verbose output about glob expansion
    let mut had_glob_output = false;
    let mut glob_output = || {
        if !had_glob_output {
            had_glob_output = true;
            verbose_println!();
        }
    };

    let glob = match path.to_str().map(Glob::new) {
        Some(Ok(glob)) => glob.compile_matcher(),
        None | Some(Err(_)) => {
            glob_output();
            verbose_println!("Could not glob-expand {}", path.display());
            return Ok(vec![PathBuf::from(path)]);
        },
    };

    let entries: Vec<walkdir::DirEntry> = WalkDir::new(dotfiles_path)
        .follow_links(true)
        .into_iter()
        .collect::<Result<_, _>>()?;

    let expanded_paths: Vec<_> = entries
        .into_iter()
        .filter_map(|entry| {
            let entry_path = entry
                .path()
                .strip_prefix(dotfiles_path)
                .expect("Entry should be in the dotfiles path");

            if glob.is_match(entry_path) {
                Some(PathBuf::from(entry_path))
            } else {
                None
            }
        })
        .collect();

    // If an entry just got expanded to itself, don't print anything about it
    match &expanded_paths.as_slice() {
        [expanded_path] if expanded_path == path => (),
        _ => {
            glob_output();
            verbose_println!("Glob-expanded {} to:", path.display());
            for expanded_path in &expanded_paths {
                verbose_println!("\t- {}", expanded_path.display())
            }
        },
    }

    Ok(expanded_paths)
}

/// Merges a partial config (obtained from the CLI and default settings) with a
/// config obtained from reading the dotrc to create a complete configuration.
fn merge_dotrc(
    partial_config: PartialConfig,
    dotrc_config: dotrc::Config,
) -> Result<Config, Error> {
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
                path.replacen("~", util::home_dir().to_str()?, 1)
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

        AbsolutePath::from(merge_hierarchy(
            partial_config.dotfiles_path,
            dotrc_dotfiles_path,
        ))
    };

    let excludes = {
        let mut excludes: Vec<AbsolutePath> =
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
            .map(|path| expand_glob(&path, &dotfiles_path))
            // If any glob expansion failed due to an I/O error, give up
            .collect::<Result<Vec<Vec<_>>, _>>()?
            // Then flatten the glob-expanded results
            .into_iter()
            .flatten()
            // Finally, make each exclude path absolute by prepending them with
            // the dotfiles path
            .map(|exclude| AbsolutePath::from(dotfiles_path.join(exclude)))
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

    let command = partial_config.command;

    Ok(Config {
        excludes,
        tags,
        dotfiles_path,
        hostname,
        command,
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
fn find_dotrc(partial_config: &PartialConfig) -> Option<AbsolutePath> {
    let config = partial_config.to_config().ok()?;

    // Try to check if a dotrc was among the files discovered from partial_config
    let items = crate::resolver::get(&config).ok()?;
    for item in items {
        match item.dest().file_name() {
            Some(name) if DOTRC_NAMES.contains(&name) => {
                verbose_println!("Discovered dotrc at {}", item.source());
                return Some(item.source().clone());
            },
            _ => (),
        }
    }

    // Otherwise, try to find a dotrc in the home directory
    for dotrc_name in DOTRC_NAMES.iter() {
        let dotrc_path = util::home_dir().join(dotrc_name);
        if dotrc_path.exists() {
            return Some(AbsolutePath::from(dotrc_path));
        }
    }

    None
}

/// Loads the complete configuration.
///
/// Draws from CLI arguments, the dotrc, and default values (where applicable)
pub fn get() -> Result<Config, Error> {
    let partial_config = PartialConfig::merge(cli::Config::get(), DefaultConfig::get()?);
    let dotrc_config = dotrc::get(find_dotrc(&partial_config))?;
    let config = merge_dotrc(partial_config, dotrc_config)?;

    Ok(config)
}

#[derive(Fail, Debug, From)]
pub enum Error {
    #[fail(display = "error reading system hostname")]
    NoSystemHostname,

    #[fail(display = "error reading file or directory ({})", _0)]
    WalkdirError(#[fail(cause)] walkdir::Error),

    #[fail(display = "{}", _0)]
    DotrcError(#[fail(cause)] dotrc::Error),
}
use self::Error::*;
