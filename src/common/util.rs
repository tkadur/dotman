use crate::common::{global::home_dir, types::FileType};

use lazy_static::lazy_static;
use parking_lot::Mutex;
use std::{
    collections::HashSet,
    ffi::OsStr,
    hash::Hash,
    io,
    path::{Path, PathBuf},
};

lazy_static! {
    pub static ref VERBOSE_PRINT_BUFFER: Mutex<Option<String>> = Mutex::new(Some(String::new()));
}

#[macro_export]
macro_rules! verbose_print {
    ($($args:tt)*) => {
        match $crate::common::global::get_verbosity() {
            // If the verbosity has not yet been set, write the string to be printed into a buffer for storage
            None => $crate::common::util::VERBOSE_PRINT_BUFFER
                .lock()
                .as_mut()
                .expect("Buffer should exist if verbosity hasn't been determined")
                .push_str(&format!($($args)*)),
            Some(verbosity) => {
                let verbose_print = |s| {
                    if verbosity {
                        print!("{}", s)
                    }
                };

                // Use and discard the buffer if this is the first invocation of `verbose_print!` since the verbosity was set
                if let Some(buf) = $crate::common::util::VERBOSE_PRINT_BUFFER .lock() .take() {
                    verbose_print(format!("{}", buf))
                }

                verbose_print(format!($($args)*));
            },
        }
    }
}

#[macro_export]
macro_rules! verbose_println {
    () => {{
        use $crate::verbose_print;
        verbose_print!("\n");
    }};
    ($($args:tt)*) => {{
        use $crate::verbose_print;
        verbose_print!($($args)*);
        verbose_print!("\n");
    }}
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

pub fn file_type(path: impl AsRef<Path>) -> io::Result<FileType> {
    let file_type = path.as_ref().symlink_metadata()?.file_type();

    Ok(if file_type.is_file() {
        FileType::File
    } else if file_type.is_dir() {
        FileType::Directory
    } else if file_type.is_symlink() {
        FileType::Symlink
    } else {
        unreachable!()
    })
}
