use serde::Deserialize;
use std::error;
use std::{fs, io::Read, path::PathBuf};

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
/// Configuration options available in rcrc
pub struct Config {
    pub excludes: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub dotfiles_path: Option<String>,
    pub hostname: Option<String>,
}

/// Gets configuration options from the rcrc file.
///
/// Failure to read from rcrc (e.g. if the file doesn't exist)
/// is _not_ considered an error, and will return an empty
/// config. A malformed rcrc, on the other hand, _is_ considered
/// an error.
pub fn get(rcrc_path: Option<PathBuf>) -> Result<Config, Box<error::Error>> {
    let path = match rcrc_path {
        Some(path) => path,
        None => return Ok(Config::default()),
    };

    let contents = {
        let mut file = match fs::File::open(path) {
            Ok(file) => file,
            Err(_) => return Ok(Config::default()),
        };

        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        contents
    };

    let config = toml::from_str::<Config>(&contents)?;

    Ok(config)
}
