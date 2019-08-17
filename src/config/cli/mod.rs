mod internal;

use crate::common::{util, Platform};
use std::{ffi::OsString, iter, path::PathBuf};
use structopt::StructOpt;

/// The portion of the configuration read from CLI arguments
#[derive(Debug, Clone)]
pub struct Config {
    /// Enables verbose output.
    pub verbose: bool,

    /// Paths (relative to the dotfiles folder) of items to be excluded.
    /// This is in addition to any excludes defined in your dotrc.
    /// Globs are accepted - just make sure to enclose them in single quotes to
    /// avoid your shell trying to expand them.
    pub excludes: Vec<PathBuf>,

    /// Tags to enable. This is in addition to any tags enabled in your dotrc.
    pub tags: Vec<String>,

    /// The folder in which to search for dotfiles. The default is ~/.dotfiles.
    pub dotfiles_path: Option<PathBuf>,

    /// The hostname to use. The default is the system hostname.
    pub hostname: Option<String>,

    /// The platform to use. The default is the actual platform.
    /// Valid values are macos, windows, linux, and wsl.
    pub platform: Option<Platform>,

    pub command: Command,
}

impl Config {
    pub fn get() -> Self {
        let app = internal::RawConfig::clap();
        let raw_config = internal::RawConfig::from_clap(&app.get_matches());

        let (command, command_options) = match raw_config.command {
            internal::Command::Ls { options } => (Ls, options),
            internal::Command::Link { dry_run, options } => (Link { dry_run }, options),
        };

        let verbose = raw_config.options.verbose || command_options.verbose;
        let excludes = util::append_vecs(raw_config.options.excludes, command_options.excludes);
        let tags = util::append_vecs(raw_config.options.tags, command_options.tags);

        /// Given the name of an argument which should be unique, tries to get
        /// it from either the main command or a subcommand. If it is
        /// provided multiple times in a way that `clap` won't catch
        /// (e.g. given once to the main command and again to a subcommand),
        /// produces an appropriate `clap` error and exits.
        macro_rules! get_unique_arg {
            ($name: ident) => {
                match (raw_config.options.$name, command_options.$name) {
                    (None, None) => None,
                    (Some($name), None) | (None, Some($name)) => Some($name),
                    (Some(_), Some(_)) => {
                        // We simply append an extra usage of --$name onto the existing args, then
                        // try to parse them again. This should trigger an error about
                        // $name appearing twice, which we display before exiting.
                        //
                        // This should make this particular hack transparent to the user, since the
                        // error is just like if `clap` had caught the error.

                        let args = std::env::args_os()
                            .chain(iter::once(OsString::from(concat!("--", stringify!($name)))));

                        internal::RawConfig::from_iter_safe(args)
                            .expect_err("This argument should not allow duplicates")
                            .exit()
                    },
                }
            };
        }

        let dotfiles_path = get_unique_arg!(dotfiles_path);
        let hostname = get_unique_arg!(hostname);
        let platform = get_unique_arg!(platform);

        let res = Config {
            verbose,
            excludes,
            tags,
            dotfiles_path,
            hostname,
            platform,
            command,
        };

        util::set_verbosity(res.verbose);

        res
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Command {
    /// Lists the active dotfiles
    Ls,

    /// Links all active dotfiles
    Link {
        /// Skips the actual linking step. Everything else (e.g. errors and
        /// prompts) remains unchanged.
        dry_run: bool,
    },
}
use Command::*;
