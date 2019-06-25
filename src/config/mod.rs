mod rcrc;

use derive_getters::Getters;
use gethostname::gethostname;
use std::{error, path::PathBuf};

/// All rcm configuration options
//
// `rcrc::Config` is more "raw" than this because it's meant to be a direct translation
// of the user's rcrc file. This type encompasses all possible configuration options,
// eliminates the optional-ness of `rcrc::Config`'s fields
#[derive(Debug, Getters)]
pub struct Config {
    verbose: bool,
    excludes: Vec<String>,
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
    excludes: Vec<String>,
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
fn get_cli() -> PartialConfig {
    let yaml = clap::load_yaml!("cli.yml");
    let matches = clap::App::from_yaml(yaml).get_matches();

    let verbose = Some(matches.is_present("verbose"));

    let values_to_vec = |name| {
        matches
            .values_of(name)
            .map(|e| e.map(String::from).collect())
    };
    let excludes = values_to_vec("excludes").unwrap_or_else(|| vec![]);
    let tags = values_to_vec("tags").unwrap_or_else(|| vec![]);

    let dotfiles_path = matches
        .value_of("dotfiles_path")
        .map(|p| (PathBuf::from(p), PartialSource::Cli));

    let hostname = matches
        .value_of("hostname")
        .map(|h| (String::from(h), PartialSource::Cli));

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
fn get_default() -> PartialConfig {
    let verbose = None;
    let excludes = vec![];
    let tags = vec![];

    let dotfiles_path = dirs::home_dir().map(|h| (h.join(".dotfiles"), PartialSource::Default));
    let hostname = gethostname()
        .to_str()
        .map(|s| (String::from(s), PartialSource::Default));

    PartialConfig {
        verbose,
        excludes,
        tags,
        dotfiles_path,
        hostname,
    }
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

    let excludes = merge_vecs(
        partial_config.excludes,
        rcrc_config.excludes.unwrap_or_else(|| vec![]),
    );
    let tags = merge_vecs(partial_config.tags, rcrc_config.tags.unwrap_or_else(|| vec![]));

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

    let dotfiles_path = {
        let rcrc_dotfiles_path = rcrc_config.dotfiles_path.map(PathBuf::from);

        merge_hierarchy(partial_config.dotfiles_path, rcrc_dotfiles_path)
            .ok_or("Dotfiles directory not found")?
    };

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

pub fn get() -> Result<Config, Box<dyn error::Error>> {
    let partial_config = merge_partial(get_cli(), get_default());
    let rcrc_config = rcrc::get(find_rcrc(&partial_config))?;

    merge_rcrc(partial_config, rcrc_config)
}
