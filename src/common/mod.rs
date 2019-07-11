pub mod util;

use contracts::*;
use derive_getters::Getters;
use std::{
    fmt::{self, Display},
    iter::IntoIterator,
    path::PathBuf,
};

/// Represents any invariants that must hold.
///
/// `invariant()` returns whether those invariants are true.
pub trait Invariant {
    fn invariant(&self) -> bool;
}

#[derive(Debug, Getters)]
pub struct Item {
    source: PathBuf,
    dest: PathBuf,
}

impl Invariant for Item {
    fn invariant(&self) -> bool {
        self.source.is_absolute() && self.dest.is_absolute()
    }
}

#[invariant(self.invariant())]
impl Item {
    #[pre(source.is_absolute() && dest.is_absolute())]
    #[post(ret.invariant())]
    pub fn new(source: PathBuf, dest: PathBuf) -> Self {
        Item { source, dest }
    }

    pub fn display_source(&self) -> String {
        format!("{}", util::home_to_tilde(&self.source).display())
    }

    pub fn display_dest(&self) -> String {
        format!("{}", util::home_to_tilde(&self.dest).display())
    }
}

#[invariant(self.invariant())]
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

#[invariant(self.invariant())]
impl FormattedItem {
    pub fn item(&self) -> &Item {
        &self.item
    }
}

impl Invariant for FormattedItem {
    fn invariant(&self) -> bool {
        self.item.invariant()
    }
}

#[invariant(self.invariant())]
impl std::ops::Deref for FormattedItem {
    type Target = Item;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

#[invariant(self.invariant())]
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

impl Invariant for FormattedItems {
    fn invariant(&self) -> bool {
        self.formatted_items.iter().all(FormattedItem::invariant)
    }
}

#[invariant(self.invariant())]
impl FormattedItems {
    #[pre(items.iter().all(Item::invariant))]
    #[post(ret.invariant())]
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
    type IntoIter = <Vec<FormattedItem> as IntoIterator>::IntoIter;
    type Item = <Vec<FormattedItem> as IntoIterator>::Item;

    #[pre(self.invariant())]
    fn into_iter(self) -> Self::IntoIter {
        self.formatted_items.into_iter()
    }
}

#[invariant(self.invariant())]
impl<'a> IntoIterator for &'a FormattedItems {
    type IntoIter = <&'a Vec<FormattedItem> as IntoIterator>::IntoIter;
    type Item = <&'a Vec<FormattedItem> as IntoIterator>::Item;

    fn into_iter(self) -> Self::IntoIter {
        self.formatted_items.iter()
    }
}

#[invariant(self.invariant())]
impl Display for FormattedItems {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.formatted_items
            .iter()
            .map(|formatted_item| writeln!(f, "{}", formatted_item))
            .collect()
    }
}
