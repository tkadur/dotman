use crate::{
    common::{util, AbsolutePath, FormattedItem, FormattedItems},
    verbose_println,
};
use derive_more::From;
use failure::*;
use std::{
    fs,
    io::{self, Write},
    path::Path,
};

enum YN {
    Yes,
    No,
}
use self::YN::*;

/// Prompts the user with `prompt` and asks for a yes/no answer.
/// Will continue asking until input resembling yes/no is given.
fn read_yes_or_no(prompt: &str) -> io::Result<YN> {
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

#[cfg(unix)]
fn symlink(source: impl AsRef<Path>, dest: impl AsRef<Path>) -> io::Result<()> {
    std::os::unix::fs::symlink(source, dest)
}

fn link_item(item: &FormattedItem, dry_run: bool) -> Result<(), Error> {
    let (source, dest) = (item.source(), item.dest());

    // Performs the actual linking after all validation
    // is finished.
    let link = |item: &FormattedItem| -> Result<(), Error> {
        verbose_println!("Linking {}", item);

        if dry_run {
            return Ok(());
        }

        fs::create_dir_all(dest.parent().unwrap_or(dest))?;
        symlink(source, dest)?;

        Ok(())
    };

    if !dest.exists() {
        link(item)?
    } else {
        match fs::read_link(dest) {
            // If the file at `dest` is already a link to source, ignore it.
            Ok(ref target) if target.as_path() == source.as_path() => {
                verbose_println!("Skipping identical {}", dest)
            },
            // If the file at `dest` is anything else, ask if it should be overwritten
            _ => {
                let prompt = format!("Overwrite {}?", dest);
                match read_yes_or_no(&prompt)? {
                    No => println!("Skipping {}", dest),
                    Yes => {
                        match util::file_type(dest)? {
                            util::FileType::File | util::FileType::Symlink => {
                                fs::remove_file(dest)?
                            },
                            // To be careful, we don't want to overwrite directories. Especially
                            // since dotman currently only links files and not whole directories.
                            // To make sure the user _absolutely_ wants to overwrite a directory
                            // with a file symlink, we ask them to delete the directory manually
                            // before running dotman.
                            util::FileType::Directory => {
                                return Err(DirectoryOverwrite(dest.clone()))
                            },
                        };
                        link(item)?;
                    },
                }
            },
        }
    }

    Ok(())
}

pub fn link_items(items: FormattedItems, dry_run: bool) -> Result<(), Error> {
    for item in &items {
        link_item(item, dry_run)?;
    }

    Ok(())
}

#[derive(Debug, From, Fail)]
pub enum Error {
    #[fail(display = "error creating symlinks ({})", _0)]
    IoError(#[fail(cause)] io::Error),

    #[fail(
        display = "won't delete directory {}. Please remove it manually if you want.",
        _0
    )]
    DirectoryOverwrite(AbsolutePath),
}
use self::Error::*;
