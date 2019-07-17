use crate::{
    common::{util, FormattedItem, FormattedItems},
    verbose_println,
};
use derive_more::From;
use std::{
    error,
    fmt::{self, Display},
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, From)]
pub enum Error {
    IoError(io::Error),
    DirectoryOverwrite(PathBuf),
}
use self::Error::*;

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let error_msg = match self {
            IoError(error) => format!("error creating symlinks ({})", error),
            DirectoryOverwrite(path) => format!(
                "won't delete directory {}. Please remove it manually if you want.",
                path.display()
            ),
        };

        write!(f, "{}", error_msg)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            IoError(error) => Some(error),
            DirectoryOverwrite(_) => None,
        }
    }
}

#[cfg(unix)]
fn symlink(source: impl AsRef<Path>, dest: impl AsRef<Path>) -> io::Result<()> {
    std::os::unix::fs::symlink(source, dest)
}

fn link_item(item: &FormattedItem, dry_run: bool) -> Result<(), Error> {
    // Performs tha actual linking after all validation
    // is finished.
    let link = |item: &FormattedItem| -> Result<(), Error> {
        verbose_println!("Linking {}", item);

        if dry_run {
            return Ok(());
        }

        let dest = item.dest();

        fs::create_dir_all(dest.parent().unwrap_or(dest))?;
        symlink(item.source(), item.dest())?;

        Ok(())
    };

    if item.dest().exists() {
        // If the file at dest is already a link to source, ignore it.
        // Else, ask if it should be overwritten.
        match fs::read_link(item.dest()) {
            Ok(ref target) if target == item.source() => {
                verbose_println!("Skipping identical {}", item.dest().display())
            },
            _ => {
                print!("Overwrite {}? (y/n) ", item.dest().display());
                io::stdout().flush()?;

                let mut buf = String::new();
                io::stdin().read_line(&mut buf)?;
                buf = buf.trim().to_lowercase();
                if buf.starts_with("yes") || "yes".starts_with(&buf) {
                    match util::file_type(item.dest())? {
                        util::FileType::Directory => {
                            return Err(DirectoryOverwrite(item.dest().clone()))
                        },
                        util::FileType::File | util::FileType::Symlink => {
                            fs::remove_file(item.dest())?
                        },
                    };
                    link(item)?;
                }
            },
        }
    } else {
        link(item)?
    }

    Ok(())
}

pub fn link_items(items: FormattedItems, dry_run: bool) -> Result<(), Error> {
    for item in &items {
        link_item(item, dry_run)?;
    }

    Ok(())
}
