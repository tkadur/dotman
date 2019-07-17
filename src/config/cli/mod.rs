use crate::common::util;
use std::path::PathBuf;
use structopt::StructOpt;

/// The portion of the configuration read from CLI arguments and
/// environment variables
#[derive(Debug, Clone, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Config {
    #[structopt(short, long)]
    pub verbose: bool,

    #[structopt(short, long, parse(from_os_str))]
    pub excludes: Vec<PathBuf>,

    #[structopt(short, long)]
    pub tags: Vec<String>,

    #[structopt(long, parse(from_os_str))]
    pub dotfiles_path: Option<PathBuf>,

    #[structopt(long)]
    pub hostname: Option<String>,

    #[structopt(subcommand)]
    pub command: Command,
}

impl Config {
    pub fn get() -> Self {
        let res = Self::from_args();
        util::set_verbosity(res.verbose);

        res
    }
}

#[derive(Debug, Clone, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Command {
    Ls,

    #[structopt(name = "link")]
    Link {
        #[structopt(long)]
        dry_run: bool,
    },
}
