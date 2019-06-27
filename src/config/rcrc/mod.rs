use derive_more::From;
use serde::Deserialize;
use std::{
    error, fmt, fs,
    io::{self, Read},
    path::PathBuf,
};

#[derive(Debug, From)]
pub enum Error {
    ParseError(toml::de::Error),
    IoError(io::Error),
}
use self::Error::*;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (error_type, error_msg) = match self {
            ParseError(error) => ("parsing .rcrc", error.to_string()),
            IoError(error) => ("reading .rcrc", error.to_string()),
        };

        write!(f, "error {} ({})", error_type, error_msg)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            ParseError(error) => Some(error),
            IoError(error) => Some(error),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
/// Configuration options available in rcrc
pub struct Config {
    pub excludes: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    #[serde(rename = "dotfiles-path")]
    pub dotfiles_path: Option<String>,
    pub hostname: Option<String>,
}

/// Gets configuration options from the rcrc file.
///
/// The rcrc file not existing is _not_ considered an error,
/// and will return an empty config. Failure to read the rcrc
/// file or a malformed rcrc, on the other hand, _is_ considered
/// an error.
pub fn get(rcrc_path: Option<PathBuf>) -> Result<Config, Error> {
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

    let config = toml::from_str(&contents)?;

    Ok(config)
}
