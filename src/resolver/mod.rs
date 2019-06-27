use super::config::Config;
use derive_more::From;
use std::{
    collections::HashSet,
    error, fmt, io, iter,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

#[derive(Debug, From)]
pub enum Error {
    IoError(io::Error),
    WalkdirError(walkdir::Error),
}
use self::Error::*;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (error_type, error_msg) = match self {
            IoError(error) => ("reading from dotfiles directory", error.to_string()),
            WalkdirError(error) => ("reading from dotfiles directory", error.to_string()),
        };

        write!(f, "error {} ({})", error_type, error_msg)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            IoError(error) => Some(error),
            WalkdirError(error) => Some(error),
        }
    }
}

/// Adds every non-hidden non-excluded file in `dir` (recursively, ignoring
/// directories) to `res`.
fn link_dir_contents(
    dir: &Path,
    excludes: &HashSet<&Path>,
    res: &mut Vec<PathBuf>,
) -> Result<(), Error> {
    fn is_not_hidden(entry: &walkdir::DirEntry) -> bool {
        entry
            .file_name()
            .to_str()
            .map(|s| entry.depth() == 0 || !s.starts_with('.'))
            .unwrap_or(false)
    }

    for entry in WalkDir::new(dir).into_iter().filter_entry(is_not_hidden) {
        let entry = entry?;
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

/// Finds the items under `path` which are to be symlinked, according to all the
/// options specified, and place then in `res`
///
/// Requires: `path` is absolute
///
/// Ensures: All paths in `res` are absolute
fn find_items(
    path: PathBuf,
    is_prefixed: &impl Fn(&Path) -> bool,
    active_prefixed_dirs: &HashSet<&Path>,
    excludes: &HashSet<&Path>,
    res: &mut Vec<PathBuf>,
) -> Result<(), Error> {
    debug_assert!(path.is_absolute());

    for entry in path.read_dir()? {
        let entry = entry?;
        let entry_path_raw = entry.path();
        let entry_path = entry_path_raw
            .strip_prefix(&path)
            .expect("entry must be within root");

        let entry_name = PathBuf::from(entry.file_name());

        /// Checks if a filename is prefixed by a '.' character.
        /// If the path cannot be read as a String, assume it isn't hidden.
        fn is_hidden(filename: &Path) -> bool {
            filename
                .to_str()
                .map(|s| s.starts_with('.'))
                .unwrap_or(false)
        }

        if is_hidden(&entry_name) || excludes.contains(entry_path) {
            continue;
        }

        if is_prefixed(&entry_name) {
            if active_prefixed_dirs.contains(entry_name.as_path()) {
                find_items(
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

    for path in res {
        debug_assert!(path.is_absolute());
    }
    Ok(())
}

pub fn get(config: Config) -> Result<Vec<PathBuf>, Error> {
    let hostname_prefix = "host-";
    let tag_prefix = "tag-";
    let prefixes = [hostname_prefix, tag_prefix];

    // Checks if a path is prefixed by any element of prefixes
    // If the path cannot be read as a String, assume it isn't.
    let is_prefixed = |filename: &Path| -> bool {
        for prefix in &prefixes {
            match filename.to_str() {
                Some(s) if s.starts_with(prefix) => return true,
                _ => (),
            }
        }

        false
    };

    let hostname_dir = PathBuf::from([hostname_prefix, config.hostname()].concat());
    let tag_dirs: Vec<PathBuf> = config
        .tags()
        .iter()
        .map(|tag| PathBuf::from([tag_prefix, tag].concat()))
        .collect();
    let active_prefixed_dirs: HashSet<&Path> = iter::once(&hostname_dir)
        .chain(tag_dirs.iter())
        .map(|p| p.as_path())
        .collect();

    let excludes = config.excludes().iter().map(|e| e.as_path()).collect();

    println!("Excludes: {:?}", excludes);

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
