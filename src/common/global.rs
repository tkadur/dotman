use crate::common::types::Platform;
use lazy_static::lazy_static;
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

static VERBOSE: AtomicBool = AtomicBool::new(false);

pub fn set_verbosity(verbosity: bool) {
    VERBOSE.store(verbosity, Ordering::SeqCst);
}

pub fn get_verbosity() -> bool {
    VERBOSE.load(Ordering::SeqCst)
}

lazy_static! {
    static ref HOME_DIR: PathBuf = match dirs::home_dir() {
        Some(home_dir) => home_dir,
        None => {
            eprintln!("Error: couldn't find home directory");
            std::process::exit(1);
        },
    };
}

pub fn home_dir() -> &'static Path {
    HOME_DIR.as_path()
}

#[cfg(target_os = "macos")]
const BASIC_PLATFORM: Platform = Platform::Macos;

#[cfg(target_os = "linux")]
const BASIC_PLATFORM: Platform = Platform::Linux;

#[cfg(target_os = "windows")]
const BASIC_PLATFORM: Platform = Platform::Windows;

lazy_static! {
    static ref WSL: bool = wsl::is_wsl();
}

pub fn platform() -> Platform {
    use Platform::*;

    if *WSL {
        Wsl
    } else {
        BASIC_PLATFORM
    }
}
