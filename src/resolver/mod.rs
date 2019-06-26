use super::config::Config;
use std::{
    collections::HashSet,
    error,
    ffi::OsString,
    fmt, io, iter,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

#[derive(Debug)]
enum Error {
    FilenameError(OsString),
    IoError(io::Error),
    WalkdirError(walkdir::Error),
}
use self::Error::*;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (error_type, error_msg) = match self {
            FilenameError(name) => ("parsing filename", format!("{:?}", name)),
            IoError(error) => ("reading from dotfiles directory", error.to_string()),
            WalkdirError(error) => ("reading from dotfiles directory", error.to_string()),
        };

        write!(f, "error {} ({})", error_type, error_msg)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            FilenameError(_) => None,
            IoError(error) => Some(error),
            WalkdirError(error) => Some(error),
        }
    }
}

/// Adds every non-hidden non-excluded file in `dir` (recursively, ignoring directories) to `res`.
fn link_dir_contents(
    dir: &Path,
    excludes: &HashSet<&Path>,
    res: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn error::Error>> {
    fn is_not_hidden(entry: &walkdir::DirEntry) -> bool {
        entry
            .file_name()
            .to_str()
            .map(|s| entry.depth() == 0 || !s.starts_with('.'))
            .unwrap_or(false)
    }

    for entry in WalkDir::new(dir).into_iter().filter_entry(is_not_hidden) {
        let entry = dbg!(entry.map_err(WalkdirError))?;
        let entry_full_path = entry.path();
        let path = match dir.parent() {
            None => entry_full_path,
            Some(parent) => entry_full_path
                .strip_prefix(parent)
                .expect("entry must be a prefix of dir"),
        };

        if is_not_hidden(&entry) && entry.file_type().is_file() && !excludes.contains(path) {
            res.push(PathBuf::from(entry_full_path));
        }
    }

    Ok(())
}

fn find_items<F>(
    config: &Config,
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
        let entry_path_raw = entry.path();
        let entry_path = entry_path_raw
            .strip_prefix(&path)
            .expect("entry must be within root");

        let name_raw = entry.file_name();
        let name = name_raw
            .to_str()
            .ok_or_else(|| FilenameError(name_raw.clone()))?;

        if name.starts_with('.') || excludes.contains(entry_path) {
            continue;
        }

        if is_prefixed(name) {
            if active_prefixed_dirs.contains(name) {
                find_items(
                    config,
                    entry.path(),
                    is_prefixed,
                    active_prefixed_dirs,
                    excludes,
                    res,
                )?;
            }
        } else {
            link_dir_contents(&entry.path(), excludes, res)?;
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

    let excludes = config.excludes().iter().map(|e| e.as_path()).collect();

    let mut res = vec![];

    find_items(
        &config,
        config.dotfiles_path().clone(),
        &is_prefixed,
        &active_prefixed_dirs,
        &excludes,
        &mut res,
    )?;

    Ok(res)
}
