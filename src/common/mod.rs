pub mod util;

use derive_getters::Getters;
use std::{
    fmt::{self, Display},
    path::PathBuf,
};

#[derive(Debug, Getters)]
pub struct Item {
    source: PathBuf,
    dest: PathBuf,
}

impl Item {
    pub fn new(source: PathBuf, dest: PathBuf) -> Self {
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
