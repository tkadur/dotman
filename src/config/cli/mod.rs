use crate::common::util;
use std::path::PathBuf;
use structopt::StructOpt;

// Any doc comment placed here will get used by `structopt` as a user-facing
// description of `dotman` in the help screen. So this has to be a regular
// comment.
//
// The portion of the configuration read from CLI arguments
#[derive(Debug, Clone, StructOpt)]
#[structopt(author = "", rename_all = "kebab-case")]
pub struct Config {
    /// Enables verbose output.
    #[structopt(short, long)]
    pub verbose: bool,

    /// Paths (relative to the dotfiles folder) of items to be excluded.
    /// This is in addition to any excludes defined in your dotrc.
    /// Globs are accepted - just make sure to enclose them in single quotes to
    /// avoid your shell trying to expand them.
    #[structopt(short, long = "exclude", number_of_values = 1, parse(from_os_str))]
    pub excludes: Vec<PathBuf>,

    /// Tags to enable. This is in addition to any tags enabled in your dotrc.
    #[structopt(short, long = "tag", number_of_values = 1)]
    pub tags: Vec<String>,

    /// The folder in which to search for dotfiles. The default is ~/.dotfiles.
    #[structopt(long, parse(from_os_str))]
    pub dotfiles_path: Option<PathBuf>,

    /// The hostname to use. Only one hostname can be used. The default is the
    /// system hostname.
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
    /// Lists the active dotfiles
    Ls,

    /// Links all active dotfiles
    #[structopt(name = "link")]
    Link {
        /// Skips the actual linking step. Everything else (e.g. errors and
        /// prompts) remains unchanged.
        #[structopt(long)]
        dry_run: bool,
    },
}
