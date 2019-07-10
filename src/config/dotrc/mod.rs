use derive_more::From;
use serde::Deserialize;
use std::{
    error,
    fmt::{self, Display},
    fs,
    io::{self, Read},
    path::Path,
};

#[derive(Debug, From)]
pub enum Error {
    ParseError(serde_yaml::Error),
    IoError(io::Error),
}
use self::Error::*;

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let error_msg = match self {
            ParseError(error) => format!("error parsing .dotrc ({})", error),
            IoError(error) => format!("error reading .dotrc ({})", error),
        };

        write!(f, "{}", error_msg)
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

#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
/// Configuration options available in dotrc
pub struct Config {
    pub excludes: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    #[serde(rename = "dotfiles-path")]
    pub dotfiles_path: Option<String>,
    pub hostname: Option<String>,
}

/// Gets configuration options from the dotrc file.
///
/// The dotrc file not existing is _not_ considered an error,
/// and will return an empty config. Failure to read the dotrc
/// file or a malformed dotrc, on the other hand, _is_ considered
/// an error.
pub fn get<P>(dotrc_path: Option<P>) -> Result<Config, Error>
where
    P: AsRef<Path>,
{
    let path = match dotrc_path {
        Some(path) => path,
        None => return Ok(Config::default()),
    };

    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return Ok(Config::default()),
    };

    let contents = {
        let mut file = file;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        contents
    };

    // serde_yaml errors on empty input, so handle that case manually
    if contents.is_empty() {
        return Ok(Config::default())
    }

    let config = serde_yaml::from_str(&contents)?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::Config;
    use pretty_assertions::assert_eq;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn empty_dotrc() {
        let mut dotrc = NamedTempFile::new().unwrap();
        write!(dotrc, "").unwrap();

        let config = super::get(Some(dotrc.path())).unwrap();
        let expected = Config::default();

        assert_eq!(config, expected);
    }

    #[test]
    fn excludes() {
        let mut dotrc = NamedTempFile::new().unwrap();
        let contents = r#"
excludes:
    - python
    - secrets
"#;
        write!(dotrc, "{}", contents).unwrap();

        let config = super::get(Some(dotrc.path())).unwrap();
        let expected = Config {
            excludes: Some(vec![String::from("python"), String::from("secrets")]),
            ..Config::default()
        };

        assert_eq!(config, expected);
    }

    #[test]
    fn tags() {
        let mut dotrc = NamedTempFile::new().unwrap();
        let contents = r#"
tags:
    - haskell
    - rust
    - vim
"#;
        write!(dotrc, "{}", contents).unwrap();

        let config = super::get(Some(dotrc.path())).unwrap();
        let expected = Config {
            tags: Some(vec![
                String::from("haskell"),
                String::from("rust"),
                String::from("vim"),
            ]),
            ..Config::default()
        };

        assert_eq!(config, expected);
    }

    #[test]
    fn dotfiles_path() {
        let mut dotrc = NamedTempFile::new().unwrap();
        let contents = r#"
dotfiles-path: ~/.top_secret
"#;
        write!(dotrc, "{}", contents).unwrap();

        let config = super::get(Some(dotrc.path())).unwrap();
        let expected = Config {
            dotfiles_path: Some(String::from("~/.top_secret")),
            ..Config::default()
        };

        assert_eq!(config, expected);
    }
}
