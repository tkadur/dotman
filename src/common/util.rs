use std::{
    io,
    ops::Drop,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

/// Efficiently appends two `Vec`s together
pub fn append_vecs<T>(x: Vec<T>, mut y: Vec<T>) -> Vec<T> {
    let mut res = x;
    res.append(&mut y);

    res
}

/// Tries to replace absolute paths of the home directory
/// with a tilde for readability. If that fails for any reason, just
/// return `path`.
pub fn home_to_tilde(path: &Path) -> PathBuf {
    let home_dir = match dirs::home_dir() {
        Some(home_dir) => home_dir,
        None => return PathBuf::from(path),
    };

    let relative_path = match path.strip_prefix(home_dir) {
        Ok(relative_path) => relative_path,
        Err(_) => return PathBuf::from(path),
    };

    PathBuf::from("~").join(relative_path)
}

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

pub struct WithVerbosity {
    old_verbosity: bool,
}

impl Drop for WithVerbosity {
    fn drop(&mut self) {
        set_verbosity(self.old_verbosity);
    }
}

/// Sets the verbosity to `verbosity` within the current scope.
/// Returns an RAII object which resets the verbosity when it gets dropped.
///
/// Example usage:
/// ```
/// set_verbosity(true);
///
/// # assert_eq!(get_verbosity(), true);
/// // Make sure verbosity is off for this part
/// {
///     let _x = with_verbosity(false);
///     # assert_eq!(get_verbosity(), false);
///
///     // Actually, turn verbosity back on for a bit
///     {
///         let _x = with_verbosity(true);
///         # assert_eq!(get_verbosity(), true);
///     }
///     // Verbosity gets turned back off
///     # assert_eq!(get_verbosity(), false);
/// }
/// // Verbosity gets turned back in
/// # assert_eq!(get_verbosity(), true);
/// ```
pub fn with_verbosity(verbosity: bool) -> WithVerbosity {
    let res = WithVerbosity {
        old_verbosity: get_verbosity(),
    };
    set_verbosity(verbosity);

    res
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
