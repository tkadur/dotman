#![warn(clippy::all)]

#[macro_use]
mod common;
mod config;
mod linker;
mod resolver;

use common::{util, FormattedItems};
use std::error;

enum Subcommand {
    Link,
    Ls,
}
use self::Subcommand::*;

impl Subcommand {
    fn from_args(args: &clap::ArgMatches) -> Subcommand {
        match args.subcommand() {
            ("link", _) => Link,
            ("ls", _) => Ls,
            _ => unreachable!(),
        }
    }
}

fn go() -> Result<(), Box<dyn error::Error>> {
    let yaml = clap::load_yaml!("cli.yml");
    let args = clap::App::from_yaml(yaml).get_matches();
    util::set_verbosity(args.is_present("verbose"));

    let config = config::get(&args)?;
    verbose_println!("");
    let items = FormattedItems::from_items(resolver::get(&config)?);
    verbose_println!("");

    match Subcommand::from_args(&args) {
        Link => linker::link_items(items)?,
        Ls => println!("{}", items),
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
