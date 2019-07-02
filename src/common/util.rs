use crate::common::Item;
use std::{
    fmt::{self, Display},
    path::{Path, PathBuf},
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

/// Just a wrapper for pretty-printing multiple `Item`s by aligning the
/// arrows in the output
struct ItemList<'a> {
    items: &'a [Item],
}

impl<'a> Display for ItemList<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (sources, dests): (Vec<_>, Vec<_>) = self
            .items
            .iter()
            .map(|item| {
                (
                    format!("{}", item.display_source()),
                    format!("{}", item.display_dest()),
                )
            })
            .unzip();

        let max_source_len = sources.iter().map(|source| source.len()).max().unwrap_or(0);

        sources
            .iter()
            .zip(dests.iter())
            .map(|(source, dest)| {
                writeln!(
                    f,
                    "{:width$}  ->    {}",
                    source,
                    dest,
                    width = max_source_len
                )
            })
            .collect()
    }
}

/// Display multiple items in a cleaner way than displaying them individually
pub fn display_items<'a>(items: &'a [Item]) -> impl Display + 'a {
    ItemList { items }
}
