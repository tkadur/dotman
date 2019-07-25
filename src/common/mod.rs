pub mod util;

use contracts::*;
use derive_getters::Getters;
use itertools::Itertools;
use std::{
    convert::{AsRef, From},
    fmt::{self, Display},
    iter::IntoIterator,
    ops::Deref,
    path::{Path, PathBuf},
};

/// Represents an (owned) path which must be absolute
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AbsolutePath {
    path: PathBuf,
}

impl Display for AbsolutePath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", util::home_to_tilde(&self.path).display())
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

impl Deref for AbsolutePath {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl AsRef<Path> for AbsolutePath {
    fn as_ref(&self) -> &Path {
        self.path.as_ref()
    }
}

/// Represents the location of a dotfile (the source) and the
/// location of the symlink pointing to the source (the destination) as a pair
/// of absolute paths to the two files.
#[derive(Debug, Getters)]
pub struct Item {
    source: AbsolutePath,
    dest: AbsolutePath,
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
/// use `FormattedItems::from_items`.
pub struct FormattedItem {
    item: Item,
    width: usize,
}

impl Deref for FormattedItem {
    type Target = Item;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl Display for FormattedItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        println!("{:width$} asdf", self.source(), width = 100);
        write!(
            f,
            "{:width$}  ->    {}",
            self.source(),
            self.dest(),
            width = self.width
        )
    }
}

// Just a convenient wrapper for multiple `FormattedItem`s
pub struct FormattedItems {
    formatted_items: Vec<FormattedItem>,
}

impl FormattedItems {
    /// Formats items in a group to ensure uniform formatting.
    ///
    /// For example:
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
            .map(|item| item.source().to_string().len())
            .max()
            .unwrap_or(0);

        let formatted_items = items
            .into_iter()
            .map(|item| FormattedItem { item, width })
            .collect();

        FormattedItems { formatted_items }
    }
}

impl IntoIterator for FormattedItems {
    type IntoIter = <Vec<FormattedItem> as IntoIterator>::IntoIter;
    type Item = <Vec<FormattedItem> as IntoIterator>::Item;

    fn into_iter(self) -> Self::IntoIter {
        self.formatted_items.into_iter()
    }
}

impl<'a> IntoIterator for &'a FormattedItems {
    type IntoIter = <&'a Vec<FormattedItem> as IntoIterator>::IntoIter;
    type Item = <&'a Vec<FormattedItem> as IntoIterator>::Item;

    fn into_iter(self) -> Self::IntoIter {
        self.formatted_items.iter()
    }
}

impl Display for FormattedItems {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.formatted_items.iter().join("\n"))
    }
}
