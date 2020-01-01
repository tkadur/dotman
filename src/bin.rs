use common::FormattedItems;
use lib::*;

fn go() -> Result<(), failure::Error> {
    let config = config::Config::get()?;
    verbose_println!();
    let items = FormattedItems::from_items(resolver::get_items(&config)?);
    verbose_println!();

    use config::cli::Command;
    match config.command {
        Command::Link { dry_run } => linker::link_items(items, dry_run)?,
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
