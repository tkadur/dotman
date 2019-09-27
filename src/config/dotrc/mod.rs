use derive_more::From;
use failure::Fail;
use serde::Deserialize;
use std::{
    fs,
    io::{self, Read},
    path::Path,
};

/// Configuration options available in dotrc
#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Config {
    pub(super) excludes: Option<Vec<String>>,
    pub(super) tags: Option<Vec<String>>,
    #[serde(rename = "dotfiles-path")]
    pub(super) dotfiles_path: Option<String>,
    pub(super) hostname: Option<String>,
    pub(super) platform: Option<String>,
}

/// Gets configuration options from the dotrc file.
///
/// The dotrc file not existing is _not_ considered an error,
/// and will return an empty config. Failure to read the dotrc
/// file or a malformed dotrc, on the other hand, _is_ considered
/// an error.
pub(super) fn get(dotrc_path: Option<impl AsRef<Path>>) -> Result<Config, Error> {
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
        return Ok(Config::default());
    }

    let config = serde_yaml::from_str(&contents)?;

    Ok(config)
}

#[derive(Debug, From, Fail)]
pub enum Error {
    #[fail(display = "error parsing .dotrc ({})", _0)]
    ParseError(#[fail(cause)] serde_yaml::Error),

    #[fail(display = "error reading .dotrc ({})", _0)]
    IoError(#[fail(cause)] io::Error),
}

#[cfg(test)]
mod tests {
    use super::Config;
    use pretty_assertions::assert_eq;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn mock_dotrc(contents: &str) -> Config {
        let mut dotrc = NamedTempFile::new().unwrap();
        write!(dotrc, "{}", contents).unwrap();

        super::get(Some(dotrc.path())).unwrap()
    }

    #[test]
    fn empty_dotrc() {
        let config = mock_dotrc("");
        let expected = Config::default();

        assert_eq!(config, expected);
    }

    #[test]
    fn excludes() {
        let contents = r#"
            excludes:
                - python
                - secrets
        "#;
        let config = mock_dotrc(contents);

        let expected = Config {
            excludes: Some(vec![String::from("python"), String::from("secrets")]),
            ..Config::default()
        };

        assert_eq!(config, expected);
    }

    #[test]
    fn tags() {
        let contents = r#"
            tags:
                - haskell
                - rust
                - vim
        "#;
        let config = mock_dotrc(contents);

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
        let contents = r#"
            dotfiles-path: ~/.top_secret
        "#;
        let config = mock_dotrc(contents);

        let expected = Config {
            dotfiles_path: Some(String::from("~/.top_secret")),
            ..Config::default()
        };

        assert_eq!(config, expected);
    }

    #[test]
    fn hostname() {
        let contents = r#"
            hostname: my-amazing-computer
        "#;
        let config = mock_dotrc(contents);

        let expected = Config {
            hostname: Some(String::from("my-amazing-computer")),
            ..Config::default()
        };

        assert_eq!(config, expected);
    }

    #[test]
    fn platform() {
        let contents = r#"
            platform: macos
        "#;
        let config = mock_dotrc(contents);

        let expected = Config {
            platform: Some(String::from("macos")),
            ..Config::default()
        };

        assert_eq!(config, expected);
    }
}
