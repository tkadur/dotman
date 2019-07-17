#![warn(clippy::all)]

#[macro_use]
mod common;
mod config;
mod linker;
mod resolver;

use common::FormattedItems;
use std::error;

fn go() -> Result<(), Box<dyn error::Error>> {
    let config = config::get()?;
    verbose_println!("");
    let items = FormattedItems::from_items(resolver::get(&config)?);
    verbose_println!("");

    use config::cli::Command;
    match config.command() {
        Command::Link { dry_run } => linker::link_items(items, *dry_run)?,
        Command::Ls => println!("{}", items),
    }

    Ok(())
}

fn main() {
    match go() {
        Ok(()) => (),
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        },
    };
}
