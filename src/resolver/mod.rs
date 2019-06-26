use super::config::Config;
use std::{
    collections::HashSet,
    error,
    ffi::OsString,
    fmt, io, iter,
    path::{Path, PathBuf},
};

#[derive(Debug)]
enum Error {
    FilenameError(OsString),
    IoError(io::Error),
}
use self::Error::*;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (error_type, error_msg) = match self {
            IoError(error) => ("reading dotfiles directory", error.to_string()),
            FilenameError(name) => ("parsing filename", format!("{:?}", name)),
        };

        write!(f, "Error {}: {}", error_type, error_msg)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            IoError(error) => Some(error),
            FilenameError(_) => None,
        }
    }
}

fn find_items<F>(
    path: PathBuf,
    is_prefixed: &F,
    active_prefixed_dirs: &HashSet<String>,
    excludes: &HashSet<&Path>,
    res: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn error::Error>>
where
    F: Fn(&str) -> bool,
{
    for entry in path.read_dir().map_err(IoError)? {
        let entry = entry.map_err(IoError)?;

        let name_raw = entry.file_name();
        let name = name_raw
            .to_str()
            .ok_or_else(|| FilenameError(name_raw.clone()))?;

        if name.starts_with('.') || excludes.contains(Path::new(name)) {
            continue;
        }

        if is_prefixed(name) {
            if active_prefixed_dirs.contains(name) {
                find_items(
                    entry.path(),
                    is_prefixed,
                    active_prefixed_dirs,
                    excludes,
                    res,
                )?;
            }
        } else {
            res.push(entry.path());
        }
    }

    Ok(())
}

pub fn get(config: Config) -> Result<Vec<PathBuf>, Box<dyn error::Error>> {
    let hostname_prefix = "host-";
    let tag_prefix = "tag-";
    let prefixes = [hostname_prefix, tag_prefix];
    let is_prefixed = |filename: &str| {
        for prefix in &prefixes {
            if filename.starts_with(prefix) {
                return true;
            }
        }

        false
    };

    let hostname_dir = [hostname_prefix, config.hostname()].concat();
    let tag_dirs = config.tags().iter().map(|tag| [tag_prefix, tag].concat());
    let active_prefixed_dirs = iter::once(hostname_dir)
        .chain(tag_dirs)
        .collect::<HashSet<_>>();

    let excludes = config
        .excludes()
        .iter()
        .map(|e| Path::new(e.as_str()))
        .collect();

    let mut res = vec![];

    find_items(
        config.dotfiles_path().clone(),
        &is_prefixed,
        &active_prefixed_dirs,
        &excludes,
        &mut res,
    )?;

    Ok(res)
}