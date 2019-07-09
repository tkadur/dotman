pub mod util;

use derive_getters::Getters;
use std::{
    fmt::{self, Display},
    iter::IntoIterator,
    path::PathBuf,
};

#[derive(Debug, Getters)]
pub struct Item {
    source: PathBuf,
    dest: PathBuf,
}

impl Item {
    pub fn new(source: PathBuf, dest: PathBuf) -> Self {
        debug_assert!(source.is_absolute());
        debug_assert!(source.is_absolute());

        Item { source, dest }
    }

    pub fn display_source(&self) -> impl Display {
        format!("{}", util::home_to_tilde(&self.source).display())
    }

    pub fn display_dest(&self) -> impl Display {
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
/// The intent is to simultaneously build all the `FormattedItem`s so
/// they all be aligned with each other.
pub struct FormattedItem {
    item: Item,
    width: usize,
}

impl FormattedItem {
    pub fn item(&self) -> &Item {
        &self.item
    }
}

impl std::ops::Deref for FormattedItem {
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
            .map(|item| format!("{}", item.display_source()).len())
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
    type IntoIter = std::vec::IntoIter<FormattedItem>;
    type Item = FormattedItem;

    fn into_iter(self) -> Self::IntoIter {
        self.formatted_items.into_iter()
    }
}

impl<'a> IntoIterator for &'a FormattedItems {
    type IntoIter = std::slice::Iter<'a, FormattedItem>;
    type Item = &'a FormattedItem;

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
