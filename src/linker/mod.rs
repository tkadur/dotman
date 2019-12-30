use crate::{
    common::{util, AbsolutePath, FormattedItem, FormattedItems, YN},
    verbose_println,
};
use derive_more::From;
use failure::Fail;
use std::{fs, io, path::Path};

#[cfg(unix)]
fn symlink(source: impl AsRef<Path>, dest: impl AsRef<Path>) -> io::Result<()> {
    std::os::unix::fs::symlink(source, dest)
}

fn link_item(formatted_item: &FormattedItem, dry_run: bool) -> Result<(), Error> {
    let (source, dest) = (&formatted_item.item().source, &formatted_item.item().dest);

    // Performs the actual linking after all validation
    // is finished.
    let link = |item: &FormattedItem| -> Result<(), Error> {
        verbose_println!("Linking {}", item);

        if !dry_run {
            fs::create_dir_all(dest.parent().unwrap_or(dest))?;
            symlink(source, dest)?;
        }

        Ok(())
    };

    if !dest.exists() {
        link(formatted_item)?
    } else {
        match fs::read_link(dest) {
            // If the file at `dest` is already a link to `source`, ignore it.
            Ok(target) if target.as_path() == source.as_path() => {
                verbose_println!("Skipping identical {}", dest)
            },
            // If the file at `dest` is anything else, ask if it should be overwritten
            _ => {
                let prompt = format!("Overwrite {}?", dest);
                match YN::read_from_cli(&prompt)? {
                    YN::No => println!("Skipping {}", dest),
                    YN::Yes => {
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
                        link(formatted_item)?;
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
use Error::*;
