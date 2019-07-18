pub mod util;

use contracts::*;
use derive_getters::Getters;
use std::{
    convert::{AsRef, From},
    fmt::{self, Display},
    iter::IntoIterator,
    ops::Deref,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AbsolutePath {
    path: PathBuf,
}

// I'd like to have a blanket `From<P> where P: AsRef<Path>` impl,
// but that won't work until you can add a `P != AbsolutePath` constraint.
// Otherwise, you run up against the blanket `From<T> for T` impl.
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

#[derive(Debug, Getters)]
pub struct Item {
    source: AbsolutePath,
    dest: AbsolutePath,
}

impl Item {
    pub fn new(source: AbsolutePath, dest: AbsolutePath) -> Self {
        Item { source, dest }
    }

    pub fn display_source(&self) -> String {
        format!("{}", util::home_to_tilde(&self.source).display())
    }

    pub fn display_dest(&self) -> String {
        format!("{}", util::home_to_tilde(&self.dest).display())
    }
}

impl Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} -> {}", self.display_source(), self.display_dest())
    }
}

/// Just a wrapper for pretty-formatting Item by aligning the
/// arrows in the `Display` impl.
///
/// This type is not meant to be constructed directly. Instead,
/// use `FormattedItems`.
pub struct FormattedItem {
    item: Item,
    width: usize,
}

impl FormattedItem {
    pub fn item(&self) -> &Item {
        &self.item
    }
}

impl Deref for FormattedItem {
    type Target = Item;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl Display for FormattedItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:width$}  ->    {}",
            self.item().display_source(),
            self.item().display_dest(),
            width = self.width
        )
    }
}

// Just a convenient wrapper for multiple `FormattedItem`s
pub struct FormattedItems {
    formatted_items: Vec<FormattedItem>,
}

impl FormattedItems {
    pub fn from_items(items: Vec<Item>) -> Self {
        let width = items
            .iter()
            .map(|item| item.display_source().len())
            .max()
            .unwrap_or(0);

        let formatted_items = items
            .into_iter()
            .map(|item| FormattedItem { item, width })
            .collect();

        FormattedItems { formatted_items }
    }

    pub fn items(&self) -> &[FormattedItem] {
        &self.formatted_items
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
        self.formatted_items
            .iter()
            .map(|formatted_item| writeln!(f, "{}", formatted_item))
            .collect()
    }
}
