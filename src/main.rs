#![warn(clippy::all)]

mod config;
mod resolver;

use std::error;

fn go() -> Result<(), Box<dyn error::Error>> {
    let yaml = clap::load_yaml!("cli.yml");
    let args = clap::App::from_yaml(yaml).get_matches();

    let config = config::get(&args)?;
    println!("{:#?}", config);

    let paths = resolver::get(config)?;
    println!("{:#?}", paths);

    match args.subcommand() {
        ("link", Some(_sub_args)) => (),
        ("ls", Some(_sub_args)) => (),
        _ => unreachable!(),
    }

    Ok(())
}

fn main() {
    match go() {
        Ok(()) => (),
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    };
}
