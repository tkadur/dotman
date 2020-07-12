use crate::common::Platform;
use lazy_static::lazy_static;
use std::{
    collections::HashSet,
    ffi::OsStr,
    hash::Hash,
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

#[cfg(target_os = "macos")]
const BASIC_PLATFORM: Platform = Platform::Macos;

#[cfg(target_os = "linux")]
const BASIC_PLATFORM: Platform = Platform::Linux;

#[cfg(target_os = "windows")]
const BASIC_PLATFORM: Platform = Platform::Windows;

lazy_static! {
    static ref WSL: bool = wsl::is_wsl();
}

pub fn platform() -> Platform {
    use Platform::*;

    if *WSL {
        Wsl
    } else {
        BASIC_PLATFORM
    }
}

/// Efficiently appends two `Vec`s together
pub fn append_vecs<T>(x: Vec<T>, mut y: Vec<T>) -> Vec<T> {
    let mut res = x;
    res.append(&mut y);

    res
}

/// Searches for an element of the iterator which appears more than once.
///
/// If no such element exists, `find_duplicate()` returns `None`.
/// Otherwise, `find_duplicate()` returns `Some(element)`, where `element` is
/// the first one which appears more than once.
pub fn find_duplicate<I>(iter: I) -> Option<I::Item>
where
    I: IntoIterator,
    I::Item: Clone + Eq + Hash,
{
    let mut seen = HashSet::new();
    for x in iter {
        if seen.contains(&x) {
            return Some(x);
        }

        seen.insert(x);
    }

    None
}

lazy_static! {
    static ref HOME_DIR: PathBuf = match dirs::home_dir() {
        Some(home_dir) => home_dir,
        None => {
            eprintln!("Error: couldn't find home directory");
            std::process::exit(1);
        },
    };
}

pub fn home_dir() -> &'static Path {
    HOME_DIR.as_path()
}

/// If `path` begins with the absolute path of the home directory, replaces it
/// with a tilde. If `path` doesn't start with the absolute path of the home
/// directory, just returns `path`.
pub fn home_to_tilde(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    match path.strip_prefix(home_dir()) {
        Ok(relative_path) => PathBuf::from("~").join(relative_path),
        // The home directory isn't a prefix of `path` - just return `path` unchanged
        Err(_) => PathBuf::from(path),
    }
}

/// If `path` begins with a tilde, expands it into the full home directory path.
/// If `path` doesn't start with a tilde, just returns `path`.
pub fn tilde_to_home(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    match path.strip_prefix("~") {
        Ok(relative_path) => home_dir().join(relative_path),
        // ~ isn't a prefix of `path` - just return `path` unchanged
        Err(_) => PathBuf::from(path),
    }
}

/// Checks if a filename is prefixed by a '.' character.
/// If the path cannot be read as UTF-8, assume it isn't hidden.
pub fn is_hidden(filename: &OsStr) -> bool {
    filename
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

#[derive(Debug, Clone, Copy)]
pub enum FileType {
    File,
    Directory,
    Symlink,
}
use FileType::*;

pub fn file_type(path: impl AsRef<Path>) -> io::Result<FileType> {
    let file_type = path.as_ref().symlink_metadata()?.file_type();

    Ok(if file_type.is_file() {
        File
    } else if file_type.is_dir() {
        Directory
    } else if file_type.is_symlink() {
        Symlink
    } else {
        unreachable!()
    })
}

static VERBOSE: AtomicBool = AtomicBool::new(false);

pub fn set_verbosity(verbosity: bool) {
    VERBOSE.store(verbosity, Ordering::SeqCst);
}

pub fn get_verbosity() -> bool {
    VERBOSE.load(Ordering::SeqCst)
}

/// Print if the verbose flag has been set.
#[macro_export]
macro_rules! verbose_print {
     ($($args:tt)*) => {
         if crate::common::util::get_verbosity() {
             print!($($args)*);
         }
     }
}

/// Print (with a newline) if the verbose flag has been set.
#[macro_export]
macro_rules! verbose_println {
     ($($args:tt)*) => {
         if crate::common::util::get_verbosity() {
             println!($($args)*);
         }
     }
}
