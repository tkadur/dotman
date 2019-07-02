#![warn(clippy::all)]

mod common;
mod config;
mod resolver;

use common::util;
use std::error;

enum Subcommand<'a> {
    Link { sub_args: &'a clap::ArgMatches<'a> },
    Ls,
}
use self::Subcommand::*;

impl<'a> Subcommand<'a> {
    fn get(args: &'a clap::ArgMatches<'a>) -> Subcommand<'a> {
        match args.subcommand() {
            ("link", Some(sub_args)) => Link { sub_args },
            ("ls", _) => Ls,
            _ => unreachable!(),
        }
    }
}

fn go() -> Result<(), Box<dyn error::Error>> {
    let yaml = clap::load_yaml!("cli.yml");
    let args = clap::App::from_yaml(yaml).get_matches();

    let config = config::get(&args)?;
    let items = resolver::get(&config)?;

    match Subcommand::get(&args) {
        Link { .. } => (),
        Ls => println!("{}", util::display_items(&items)),
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
