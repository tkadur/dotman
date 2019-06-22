use derive_more::From;
use serde::Deserialize;
use std::{
    error, fmt, fs,
    io::{self, Read},
};

const RCRC_FILENAME: &'static str = ".rcrc-test";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
/// Configuration options available in rcrc
pub struct Config {
    excludes: Option<Vec<String>>,
    tags: Option<Vec<String>>,
    dotfiles_path: Option<String>,
    hostname: Option<String>,
}

#[derive(Debug, From)]
pub enum Error {
    ParseError(toml::de::Error),
    IoError(io::Error),
}
use self::Error::*;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (error_type, error_msg) = match self {
            ParseError(error) => ("Parse error", error.to_string()),
            IoError(error) => ("I/O error", error.to_string()),
        };

        write!(f, "{}: {}", error_type, error_msg)
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

pub fn get() -> Result<Config, Error> {
    let path = {
        let mut path = dirs::home_dir().ok_or(io::Error::new(
            io::ErrorKind::NotFound,
            "Could not find home directory",
        ))?;
        path.push(RCRC_FILENAME);

        path
    };

    let contents = {
        let mut file = fs::File::open(path)?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        contents
    };

    let config = toml::from_str::<Config>(&contents)?;

    Ok(config)
}
