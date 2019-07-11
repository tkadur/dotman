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
    path::PathBuf,
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

#[derive(Debug)]
struct Config {
    dry_run: bool,
}

#[cfg(unix)]
fn link(item: &FormattedItem, config: &Config) -> Result<(), Error> {
    verbose_println!("Linking {}", item);

    if config.dry_run {
        return Ok(());
    }

    let dest = item.dest();

    fs::create_dir_all(dest.parent().unwrap_or(dest))?;
    std::os::unix::fs::symlink(item.source(), item.dest())?;

    Ok(())
}

fn link_item(item: &FormattedItem, config: &Config) -> Result<(), Error> {
    let (source, dest) = (item.source(), item.dest());

    if dest.exists() {
        // If the file at dest is already a link to source, ignore it.
        // Else, ask if it should be overwritten.
        match fs::read_link(dest) {
            Ok(ref target) if target == source => {
                verbose_println!("Skipping identical {}", dest.display())
            },
            _ => {
                print!("Overwrite {}? (y/n) ", dest.display());
                io::stdout().flush()?;

                let mut buf = String::new();
                io::stdin().read_line(&mut buf)?;
                buf = buf.trim().to_lowercase();
                if buf.starts_with("yes") || "yes".starts_with(&buf) {
                    match util::file_type(dest)? {
                        util::FileType::Directory => return Err(DirectoryOverwrite(dest.clone())),
                        util::FileType::File | util::FileType::Symlink => fs::remove_file(dest)?,
                    };
                    link(item, config)?;
                }
            },
        }
    } else {
        link(item, config)?
    }

    Ok(())
}

pub fn link_items(items: FormattedItems, args: &clap::ArgMatches) -> Result<(), Error> {
    let dry_run = args.is_present("dry_run");
    let config = Config{ dry_run};

    for item in &items {
        link_item(item, &config)?;
    }

    Ok(())
}
