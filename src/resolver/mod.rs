use crate::{common::Item, config::Config, verbose_println};
use derive_more::From;
use std::{
    collections::HashSet,
    error,
    ffi::OsString,
    fmt::{self, Display},
    io, iter,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

#[derive(Debug, From)]
pub enum Error {
    NoHomeDirectory,
    /// Indicates when there are multiple active sources pointing to the same
    /// destination.
    DuplicateFiles {
        dest: PathBuf,
    },
    IoError(io::Error),
    WalkdirError(walkdir::Error),
}
use self::Error::*;

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let error_msg = match self {
            NoHomeDirectory => String::from("can't find home directory"),
            DuplicateFiles { dest } => {
                format!("multiple source files for destination {}", dest.display())
            },
            IoError(error) => format!(
                "error eading from dotfiles directory ({})",
                error.to_string()
            ),
            WalkdirError(error) => format!(
                "error reading from dotfiles directory ({})",
                error.to_string()
            ),
        };

        write!(f, "{}", error_msg)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            NoHomeDirectory | DuplicateFiles { .. } => None,
            IoError(error) => Some(error),
            WalkdirError(error) => Some(error),
        }
    }
}

/// Returns every non-hidden non-excluded file in `dir` (recursively, ignoring
/// directories).
///
/// Requires: `dir` is absolute
fn link_dir_contents(dir: &Path, excludes: &HashSet<&Path>) -> Result<Vec<Item>, Error> {
    debug_assert!(dir.is_absolute());

    /// Checks if a entry's filename is not prefixed by a '.' character.
    /// If the path cannot be read as a String, assume it isn't hidden.
    fn is_not_hidden(entry: &walkdir::DirEntry) -> bool {
        entry
            .file_name()
            .to_str()
            .map(|s| entry.depth() == 0 || !s.starts_with('.'))
            .unwrap_or(false)
    }

    fn make_hidden(path: &Path) -> PathBuf {
        let path_str = OsString::from(path.as_os_str());
        let hidden_path = {
            let mut hidden_path = OsString::from(".");
            hidden_path.push(path_str);

            hidden_path
        };

        PathBuf::from(hidden_path)
    }

    let mut res = vec![];
    for entry in WalkDir::new(dir).into_iter().filter_entry(is_not_hidden) {
        let entry = entry?;

        let path = entry.path();

        if excludes.contains(path) {
            verbose_println!("Excluded {}", path.display());
        }

        if is_not_hidden(&entry) && entry.file_type().is_file() && !excludes.contains(path) {
            let source = PathBuf::from(path);
            let dest = {
                let dest_tail = match dir.parent() {
                    None => path,
                    Some(parent) => path
                        .strip_prefix(parent)
                        .expect("dir must be a prefix of entry"),
                };
                dirs::home_dir()
                    .ok_or(NoHomeDirectory)?
                    .join(make_hidden(dest_tail))
            };
            res.push(Item::new(source, dest));
        }
    }

    for item in &res {
        debug_assert!(item.source().is_absolute());
        debug_assert!(item.dest().is_absolute());
    }
    Ok(res)
}

/// Finds the items under `path` which are to be symlinked, according to all the
/// options specified, and place then in `res`
///
/// Requires: `path` is absolute
///
/// Ensures: All paths in the output are absolute
fn find_items(
    root: PathBuf,
    is_prefixed: &impl Fn(&Path) -> bool,
    active_prefixed_dirs: &HashSet<&Path>,
    excludes: &HashSet<&Path>,
    res: &mut Vec<Item>,
) -> Result<(), Error> {
    debug_assert!(root.is_absolute());

    for entry in root.read_dir()? {
        let entry = entry?;
        let path = entry.path();
        debug_assert!(path.is_absolute());

        let entry_name = PathBuf::from(entry.file_name());

        /// Checks if a filename is prefixed by a '.' character.
        /// If the path cannot be read as a String, assume it isn't hidden.
        fn is_hidden(filename: &Path) -> bool {
            filename
                .to_str()
                .map(|s| s.starts_with('.'))
                .unwrap_or(false)
        }

        let excluded = excludes.contains(path.as_path());
        if is_hidden(&entry_name) || excluded {
            if excluded {
                verbose_println!("Excluded {}", path.display());
            }
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
            let contents = link_dir_contents(&entry.path(), excludes)?;
            res.extend(contents);
        }
    }

    for item in res {
        debug_assert!(item.source().is_absolute());
        debug_assert!(item.dest().is_absolute());
    }
    Ok(())
}

pub fn get(config: &Config) -> Result<Vec<Item>, Error> {
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

    let mut res = vec![];

    find_items(
        config.dotfiles_path().clone(),
        &is_prefixed,
        &active_prefixed_dirs,
        &excludes,
        &mut res,
    )?;

    // Check for duplicate destinations
    let mut seen = HashSet::new();
    for item in &res {
        let dest = item.dest().clone();
        if seen.contains(&dest) {
            return Err(DuplicateFiles { dest });
        } else {
            seen.insert(dest);
        }
    }

    Ok(res)
}
