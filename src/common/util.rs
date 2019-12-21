use crate::common::Platform;
use lazy_static::lazy_static;
use std::{
    ffi::OsStr,
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

/// Tries to replace absolute paths of the home directory
/// with a tilde for readability. If that fails for any reason, just
/// return `path`.
pub fn home_to_tilde(path: &Path) -> PathBuf {
    match path.strip_prefix(home_dir()) {
        Ok(relative_path) => PathBuf::from("~").join(relative_path),
        // The home directory isn't a prefix of `path` - just return `path` unchanged
        Err(_) => return PathBuf::from(path),
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

#[macro_export]
macro_rules! verbose_print {
     ($($args:tt)*) => {
         if crate::common::util::get_verbosity() {
             print!($($args)*);
         }
     }
}

#[macro_export]
macro_rules! verbose_println {
     ($($args:tt)*) => {
         if crate::common::util::get_verbosity() {
             println!($($args)*);
         }
     }
}
