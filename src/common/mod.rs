pub mod util;

use contracts::*;
use derive_more::{AsRef, Deref, IntoIterator};
use failure::Fail;
use itertools::Itertools;
use std::{
    convert::From,
    fmt::{self, Display},
    io::{self, Write},
    path::{Path, PathBuf},
    str::FromStr,
};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

/// The platforms that `dotman` distinguishes between.
///
/// Note that `Linux` and `Wsl` are distinct - WSL platforms
/// are not considred Linux by `dotman`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
pub enum Platform {
    Windows,
    Macos,
    Linux,
    Wsl,
}
use Platform::*;

impl Platform {
    /// Returns the valid strings corresponding to `self`
    pub fn strs(&self) -> &[&'static str] {
        match self {
            Windows => &["win", "windows"],
            Macos => &["mac", "macos"],
            Linux => &["linux"],
            Wsl => &["wsl"],
        }
    }
}

#[derive(Debug, Fail)]
#[fail(display = "unsupported platform \"{}\"", input)]
pub struct PlatformParseError {
    input: String,
}

impl FromStr for Platform {
    type Err = PlatformParseError;

    fn from_str(s: &str) -> Result<Platform, Self::Err> {
        let s = s.trim().to_lowercase();

        for platform in Platform::iter() {
            if platform.strs().contains(&s.as_str()) {
                return Ok(platform);
            }
        }

        Err(PlatformParseError { input: s })
    }
}

/// Represents a yes/no value
#[derive(Clone, Copy, Debug)]
pub enum YN {
    Yes,
    No,
}
use YN::*;

impl YN {
    /// Prompts the user with `prompt` and asks for a yes/no answer.
    /// Will continue asking until input resembling yes/no is given.
    pub fn read_from_cli(prompt: &str) -> io::Result<Self> {
        let mut buf = String::new();
        loop {
            print!("{} (y/n) ", prompt);
            io::stdout().flush()?;

            io::stdin().read_line(&mut buf)?;
            buf = buf.trim().to_lowercase();

            if buf.is_empty() {
                continue;
            }

            if buf.starts_with("yes") || "yes".starts_with(&buf) {
                return Ok(Yes);
            } else if buf.starts_with("no") || "no".starts_with(&buf) {
                return Ok(No);
            } else {
                buf.clear();
                continue;
            }
        }
    }
}

/// Represents an (owned) path which must be absolute
#[derive(AsRef, Clone, Debug, Deref, Eq, Hash, PartialEq)]
#[as_ref(forward)]
pub struct AbsolutePath {
    path: PathBuf,
}

impl Display for AbsolutePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(&util::home_to_tilde(&self.path).display().to_string())
    }
}

// I'd like to have a blanket `impl From<P> where P: AsRef<Path> for
// AbsolutePath`, but that won't work until you can add a `P != AbsolutePath`
// constraint. Otherwise, you run up against the blanket `impl From<T> for T`.
// See https://github.com/rust-lang/rfcs/issues/1834.
//
// For now, I'll just have to deal with writing relevant impls by hand.

impl From<PathBuf> for AbsolutePath {
    #[pre(path.is_absolute())]
    fn from(path: PathBuf) -> Self {
        AbsolutePath { path }
    }
}

impl From<&Path> for AbsolutePath {
    #[pre(path.is_absolute())]
    fn from(path: &Path) -> Self {
        AbsolutePath {
            path: path.to_path_buf(),
        }
    }
}

impl From<&str> for AbsolutePath {
    fn from(path: &str) -> Self {
        AbsolutePath::from(Path::new(path))
    }
}

/// Represents the location of a dotfile (the source) and the
/// location of the symlink pointing to the source (the destination) as a pair
/// of absolute paths to the two files.
#[derive(Debug)]
pub struct Item {
    pub source: AbsolutePath,
    pub dest: AbsolutePath,
}

impl Item {
    pub fn new(source: impl Into<AbsolutePath>, dest: impl Into<AbsolutePath>) -> Self {
        Item {
            source: source.into(),
            dest: dest.into(),
        }
    }
}

/// Just a wrapper for pretty-printing `Item`s
///
/// This type is not meant to be constructed directly. Instead,
/// use `FormattedItems::from_items` to construct a collection of
/// `FormattedItem`s.
#[derive(Debug)]
pub struct FormattedItem {
    item: Item,
    width: usize,
}

impl FormattedItem {
    /// Gets the underlying item
    pub fn item(&self) -> &Item {
        &self.item
    }
}

impl Display for FormattedItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(&format!(
            "{:width$}  ->    {}",
            self.item.source,
            self.item.dest,
            width = self.width
        ))
    }
}

/// Allows for formatting multiple items in a group to ensure uniform
/// formatting.
#[derive(Debug, IntoIterator)]
pub struct FormattedItems {
    #[into_iterator(owned, ref)]
    formatted_items: Vec<FormattedItem>,
}

impl FormattedItems {
    /// Create `FormattedItems` from a collection of `Item`s.
    ///
    /// # Examples
    /// ```
    /// # use crate::lib::common::{Item, FormattedItems};
    /// let items_short = vec![
    ///     Item::new("/home/tkadur/.dotfiles/file1", "/home/tkadur/.file1"),
    ///     Item::new("/home/tkadur/.dotfiles/file2", "/home/tkadur/.file2"),
    /// ];
    ///
    /// # let str_short_expected = [
    /// #     "/home/tkadur/.dotfiles/file1  ->    /home/tkadur/.file1",
    /// #     "/home/tkadur/.dotfiles/file2  ->    /home/tkadur/.file2"
    /// # ].join("\n");
    ///
    /// // Produces the following:
    /// //
    /// // /home/tkadur/.dotfiles/file1  ->    /home/tkadur/.file1
    /// // /home/tkadur/.dotfiles/file2  ->    /home/tkadur/.file2
    /// let str_short = FormattedItems::from_items(items_short).to_string();
    ///
    /// # assert_eq!(
    /// # str_short_expected,
    /// #     str_short,
    /// # );
    ///
    /// let items_long = vec![
    ///     Item::new("/home/tkadur/.dotfiles/file1", "/home/tkadur/.file1"),
    ///     Item::new("/home/tkadur/.dotfiles/file2", "/home/tkadur/.file2"),
    ///     Item::new(
    ///         "/home/tkadur/.dotfiles/file_long",
    ///         "/home/tkadur/.file_long",
    ///     ),
    ///     Item::new(
    ///         "/home/tkadur/.dotfiles/file_even_longer",
    ///         "/home/tkadur/.file_even_longer",
    ///     ),
    /// ];
    ///
    /// // Produces the following:
    /// //
    /// // /home/tkadur/.dotfiles/file1             ->    /home/tkadur/.file1
    /// // /home/tkadur/.dotfiles/file2             ->    /home/tkadur/.file2
    /// // /home/tkadur/.dotfiles/file_long         ->    /home/tkadur/.file_long
    /// // /home/tkadur/.dotfiles/file_even_longer  ->    /home/tkadur/.file_even_longer
    /// let str_long = FormattedItems::from_items(items_long).to_string();
    ///
    /// # let str_long_expected = [
    /// #     "/home/tkadur/.dotfiles/file1             ->    /home/tkadur/.file1",
    /// #     "/home/tkadur/.dotfiles/file2             ->    /home/tkadur/.file2",
    /// #     "/home/tkadur/.dotfiles/file_long         ->    /home/tkadur/.file_long",
    /// #     "/home/tkadur/.dotfiles/file_even_longer  ->    /home/tkadur/.file_even_longer",
    /// # ]
    /// # .join("\n");
    ///
    /// # assert_eq!(
    /// #     str_long_expected,
    /// #     str_long,
    /// # );
    /// ```
    pub fn from_items(items: Vec<Item>) -> Self {
        let width = items
            .iter()
            .map(|item| item.source.to_string().len())
            .max()
            .unwrap_or(0);

        let formatted_items = items
            .into_iter()
            .map(|item| FormattedItem { item, width })
            .collect();

        FormattedItems { formatted_items }
    }
}

impl Display for FormattedItems {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(&self.formatted_items.iter().join("\n"))
    }
}
