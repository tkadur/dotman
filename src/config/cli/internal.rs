use crate::common::Platform;
use std::path::PathBuf;
use structopt::StructOpt;

// Any doc comment placed here will get used by `structopt` as a user-facing
// description of `dotman` in the help screen. So this has to be a regular
// comment.
//
// The portion of the configuration read from CLI arguments
#[derive(Debug, Clone, StructOpt)]
pub(super) struct RawConfig {
    #[structopt(subcommand)]
    pub(super) command: Command,

    #[structopt(flatten)]
    pub(super) options: Options,
}

#[derive(Debug, Clone, StructOpt)]
pub(super) enum Command {
    /// Lists the active dotfiles
    Ls {
        #[structopt(flatten)]
        options: Options,
    },

    /// Links all active dotfiles
    Link {
        /// Skips the actual linking step. Everything else (e.g. errors and
        /// prompts) remains unchanged.
        #[structopt(long)]
        dry_run: bool,

        #[structopt(flatten)]
        options: Options,
    },
}

#[derive(Debug, Clone, StructOpt)]
pub(super) struct Options {
    /// Enables verbose output.
    #[structopt(short, long)]
    pub(super) verbose: bool,

    /// Paths (relative to the dotfiles folder) of items to be excluded.
    /// This is in addition to any excludes defined in your dotrc.
    /// Globs are accepted - just make sure to enclose them in single quotes to
    /// avoid your shell trying to expand them.
    #[structopt(short, long = "exclude", number_of_values = 1, parse(from_os_str))]
    pub(super) excludes: Vec<PathBuf>,

    /// Tags to enable. This is in addition to any tags enabled in your dotrc.
    #[structopt(short, long = "tag", number_of_values = 1)]
    pub(super) tags: Vec<String>,

    /// The folder in which to search for dotfiles. The default is ~/.dotfiles.
    #[structopt(long, parse(from_os_str))]
    pub(super) dotfiles_path: Option<PathBuf>,

    /// The hostname to use. The default is the system hostname.
    #[structopt(long)]
    pub(super) hostname: Option<String>,

    /// The platform to use. The default is the actual platform.
    /// Valid values are macos, windows, linux, and wsl.
    #[structopt(long, parse(try_from_str))]
    pub(super) platform: Option<Platform>,
}
