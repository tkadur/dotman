mod rcrc;

use std::path;

#[derive(Debug)]
/// All rcm configuration options
/// 
/// `rcrc::Config` is more "raw" than this because it's meant to be a direct translation
/// of the user's rcrc file. This type encompasses all possible configuration options,
/// eliminates the optional-ness of `rcrc::Config`'s fields
pub struct Config {
    excludes: Vec<String>,
    tags: Vec<String>,
    dotfiles_path: path::PathBuf,
    hostname: String,
}