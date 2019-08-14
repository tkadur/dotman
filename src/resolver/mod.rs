use crate::{
    common::{util, AbsolutePath, Item},
    config::Config,
    verbose_println,
};
use derive_more::From;
use failure::Fail;
use std::{
    collections::HashSet,
    ffi::OsString,
    io, iter,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

/// Appends a "." to the start of `path`
fn make_hidden(path: &Path) -> PathBuf {
    let path_str = OsString::from(path.as_os_str());
    let hidden_path = {
        let mut hidden_path = OsString::from(".");
        hidden_path.push(path_str);

        hidden_path
    };

    PathBuf::from(hidden_path)
}

/// Returns every non-hidden non-excluded file in `dir` (recursively, ignoring
/// directories).
fn link_dir_contents(
    dir: &AbsolutePath,
    excludes: &HashSet<&AbsolutePath>,
) -> Result<Vec<Item>, Error> {
    let mut res = vec![];
    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_entry(|entry| !util::is_hidden(entry.file_name()))
    {
        let entry = entry?;

        let path = AbsolutePath::from(entry.path());

        if excludes.contains(&path) {
            verbose_println!("Excluded {}", path);
        }

        if !util::is_hidden(entry.file_name())
            && entry.file_type().is_file()
            && !excludes.contains(&path)
        {
            let dest = {
                let dest_tail = match dir.parent() {
                    None => path.as_path(),
                    Some(parent) => path
                        .strip_prefix(parent)
                        .expect("dir must be a prefix of entry"),
                };

                AbsolutePath::from(util::home_dir().join(make_hidden(dest_tail)))
            };
            let source = path;

            res.push(Item::new(source, dest));
        }
    }

    Ok(res)
}

/// Finds the items under `path` which are to be symlinked, according to all the
/// options specified, and place then in `res`
fn find_items(
    root: AbsolutePath,
    is_prefixed: &impl Fn(&Path) -> bool,
    active_prefixed_dirs: &HashSet<&Path>,
    excludes: &HashSet<&AbsolutePath>,
    res: &mut Vec<Item>,
) -> Result<(), Error> {
    for entry in root.read_dir()? {
        let entry = entry?;
        let path = AbsolutePath::from(entry.path());

        let entry_name = entry.file_name();
        let entry_name = Path::new(&entry_name);

        let excluded = excludes.contains(&path);
        if util::is_hidden(entry_name.as_os_str()) || excluded {
            if excluded {
                verbose_println!("Excluded {}", path);
            }
            continue;
        }

        if is_prefixed(&entry_name) {
            if active_prefixed_dirs.contains(entry_name) {
                find_items(path, is_prefixed, active_prefixed_dirs, excludes, res)?;
            }
        } else {
            let contents = link_dir_contents(&AbsolutePath::from(entry.path()), excludes)?;
            res.extend(contents);
        }
    }

    Ok(())
}

pub fn get(config: &Config) -> Result<Vec<Item>, Error> {
    let hostname_prefix = "host-";
    let tag_prefix = "tag-";
    let platform_prefix = "platform-";
    let prefixes = [hostname_prefix, tag_prefix, platform_prefix];

    // Checks if a path is prefixed by any element of `prefixes`
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

    let platform_dirs: Vec<PathBuf> = config
        .platform()
        .strs()
        .iter()
        .map(|platform| PathBuf::from([platform_prefix, platform].concat()))
        .collect();

    let tag_dirs: Vec<PathBuf> = config
        .tags()
        .iter()
        .map(|tag| PathBuf::from([tag_prefix, tag].concat()))
        .collect();

    let active_prefixed_dirs: HashSet<&Path> = iter::once(&hostname_dir)
        .chain(tag_dirs.iter())
        .chain(platform_dirs.iter())
        .map(|p| p.as_path())
        .collect();

    let excludes = config.excludes().iter().collect();

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
        let dest = item.dest();
        if seen.contains(dest) {
            return Err(DuplicateFiles { dest: dest.clone() });
        } else {
            seen.insert(dest);
        }
    }

    Ok(res)
}

#[derive(Debug, From, Fail)]
pub enum Error {
    /// Indicates when there are multiple active sources pointing to the same
    /// destination.
    #[fail(display = "multiple source files for destination {}", dest)]
    DuplicateFiles { dest: AbsolutePath },

    #[fail(display = "error reading from dotfiles directory ({})", _0)]
    IoError(#[fail(cause)] io::Error),

    #[fail(display = "error reading from dotfiles directory ({})", _0)]
    WalkdirError(#[fail(cause)] walkdir::Error),
}
use self::Error::*;
