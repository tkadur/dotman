#![warn(clippy::all)]
#![warn(
    missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub
)]
#![deny(future_incompatible, unsafe_code)]

#[macro_use]
pub mod common;
pub mod config;
pub mod linker;
pub mod resolver;
