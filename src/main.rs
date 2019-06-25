#![warn(clippy::all)]

mod config;
mod resolver;

use std::error;

fn go() -> Result<(), Box<dyn error::Error>> {
    let config = config::get()?;
    println!("{:#?}", config);
    let paths = resolver::get(config);
    println!("{:?}", paths);

    Ok(())
}

fn main() {
    match go() {
        Ok(()) => (),
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    };
}
