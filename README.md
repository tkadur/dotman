# dotman

[![crates.io](https://img.shields.io/crates/v/dotman.svg)](https://crates.io/crates/dotman)

`dotman` is a tool for managing your dotfiles.

You can see an example of dotfiles managed using `dotman` [here](https://github.com/tkadur/dotfiles).

## Installation

### From source

You can use [`cargo`](https://github.com/rust-lang/cargo) to build and install with the following command:

```sh
cargo install dotman
```

## Usage

### Command-line options

```
USAGE:
    dot [FLAGS] [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Enables verbose output.

OPTIONS:
        --dotfiles-path <dotfiles-path>    The folder in which to search for dotfiles. The default is ~/.dotfiles.
    -e, --exclude <excludes>...            Paths (relative to the dotfiles folder) of items to be excluded. This is in
                                           addition to any excludes defined in your dotrc. Globs are accepted - just
                                           make sure to enclose them in single quotes to avoid your shell trying to
                                           expand them.
        --hostname <hostname>              The hostname to use. Only one hostname can be used. The default is the system
                                           hostname.
    -t, --tag <tags>...                    Tags to enable. This is in addition to any tags enabled in your dotrc.

SUBCOMMANDS:
    help    Prints this message or the help of the given subcommand(s)
    link    Links all active dotfiles
    ls      Lists the active dotfiles
```
