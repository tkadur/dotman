#[allow(dead_code, unused_variables)]
mod config;

use std::error;

fn go() -> Result<(), Box<error::Error>> {
    let config = config::get()?;
    println!("{:#?}", config);

    Ok(())
}

fn main() {
    match go() {
        Ok(()) => (),
        Err(err) => {
            println!("{}", err);
            std::process::exit(1);
        }
    };
}
