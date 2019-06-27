mod rcrc;

use derive_getters::Getters;
use gethostname::gethostname;
use globset::Glob;
use std::{collections::HashSet, error, path::PathBuf};
use walkdir::WalkDir;

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

/// Represents a "partial" or "incomplete" version of `Config`. Fields which
/// can be merged losslessly with other `PartialConfig`s and have some notion
/// of a "default" or "empty" state (e.g. `excludes`) are not optional.
/// All other fields are optional.
#[derive(Debug)]
struct PartialConfig {
    verbose: Option<bool>,
    excludes: Vec<PathBuf>,
    tags: Vec<String>,
    dotfiles_path: Option<(PathBuf, PartialSource)>,
    hostname: Option<(String, PartialSource)>,
}

#[derive(Debug)]
enum PartialSource {
    Cli,
    Default,
}

/// Gets a partial configuration from CLI arguments.
fn get_cli(args: &clap::ArgMatches) -> PartialConfig {
    let verbose = Some(args.is_present("verbose"));

    fn values_to_vec<'a, T>(
        args: &'a clap::ArgMatches,
        name: &str,
        f: impl Fn(&'a str) -> T,
    ) -> Vec<T> {
        args.values_of(name)
            .map(|e| e.map(f).collect())
            .unwrap_or_else(|| vec![])
    }
    let excludes = values_to_vec(args, "excludes", PathBuf::from);
    let tags = values_to_vec(args, "tags", String::from);

    fn value_default<'a, T>(
        args: &'a clap::ArgMatches,
        name: &str,
        f: impl Fn(&'a str) -> T,
    ) -> Option<(T, PartialSource)> {
        args.value_of(name).map(|p| (f(p), PartialSource::Cli))
    }
    let dotfiles_path = value_default(args, "dotfiles_path", PathBuf::from);
    let hostname = value_default(args, "hostname", String::from);

    PartialConfig {
        verbose,
        excludes,
        tags,
        dotfiles_path,
        hostname,
    }
}

// TODO Consider making this a Default impl for PartialConfig
/// Gets a partial configuration corresponding to the "default" values/sources
/// of each configuration option.
fn get_default() -> Result<PartialConfig, Box<dyn error::Error>> {
    let verbose = None;
    let excludes = vec![];
    let tags = vec![];

    let dotfiles_path = Some((
        dirs::home_dir()
            .ok_or("can't find home directory")?
            .join("dotfiles"),
        PartialSource::Default,
    ));
    let hostname = Some((
        gethostname()
            .to_str()
            .ok_or("can't retrieve system hostname")?
            .to_owned(),
        PartialSource::Default,
    ));

    Ok(PartialConfig {
        verbose,
        excludes,
        tags,
        dotfiles_path,
        hostname,
    })
}

fn merge_vecs<T>(x: Vec<T>, mut y: Vec<T>) -> Vec<T> {
    let mut res = x;
    res.append(&mut y);

    res
}

/// Merges two partial configs together.
/// For fields which cannot be merged and which are present in both arguments,
/// the value from `config1` is used.
fn merge_partial(config1: PartialConfig, config2: PartialConfig) -> PartialConfig {
    let verbose = config1.verbose.or(config2.verbose);
    let excludes = merge_vecs(config1.excludes, config2.excludes);
    let tags = merge_vecs(config1.tags, config2.tags);
    let dotfiles_path = config1.dotfiles_path.or(config2.dotfiles_path);
    let hostname = config1.hostname.or(config2.hostname);

    PartialConfig {
        verbose,
        excludes,
        tags,
        dotfiles_path,
        hostname,
    }
}

fn merge_rcrc(
    partial_config: PartialConfig,
    rcrc_config: rcrc::Config,
) -> Result<Config, Box<dyn error::Error>> {
    let verbose = partial_config.verbose.unwrap_or(false);

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
        let rcrc_dotfiles_path = rcrc_config
            .dotfiles_path
            .and_then(expand_tilde)
            .map(PathBuf::from);

        merge_hierarchy(partial_config.dotfiles_path, rcrc_dotfiles_path)
            .ok_or("Dotfiles directory not found")?
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

    // Making sure to respect the hierarchy of selecting in the following order
    // - CLI
    // - rcrc
    // - Default source
    fn merge_hierarchy<T>(partial: Option<(T, PartialSource)>, rcrc: Option<T>) -> Option<T> {
        match partial {
            Some((x, PartialSource::Cli)) => Some(x),
            Some((x, PartialSource::Default)) => Some(rcrc.unwrap_or(x)),
            None => rcrc,
        }
    }

    let hostname = merge_hierarchy(partial_config.hostname, rcrc_config.hostname)
        .ok_or("Couldn't get hostname")?;

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

pub fn get(cli_args: &clap::ArgMatches) -> Result<Config, Box<dyn error::Error>> {
    let partial_config = merge_partial(get_cli(cli_args), get_default()?);
    let rcrc_config = rcrc::get(find_rcrc(&partial_config))?;

    merge_rcrc(partial_config, rcrc_config)
}
