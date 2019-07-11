// #![warn(clippy::all)]

#[macro_use]
mod common;
mod config;
mod linker;
mod resolver;

use common::{util, FormattedItems};
use std::error;

enum Subcommand<'a> {
    Link { sub_args: &'a clap::ArgMatches<'a> },
    Ls,
}
use self::Subcommand::*;

impl<'a> Subcommand<'a> {
    fn from_args(args: &'a clap::ArgMatches<'a>) -> Subcommand<'a> {
        match args.subcommand() {
            ("link", Some(sub_args)) => Link { sub_args },
            ("ls", Some(_)) => Ls,
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
        Link { sub_args } => linker::link_items(items, sub_args)?,
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
