#![deny(unreachable_patterns)]
#![deny(unknown_lints)]
#![cfg_attr(not(debug_assertions), deny(unused_variables))]
#![cfg_attr(not(debug_assertions), deny(unused_imports))]
#![cfg_attr(not(debug_assertions), deny(dead_code))]
#![deny(unused_must_use)]
#![cfg_attr(not(debug_assertions), deny(unused_extern_crates))]
#![feature(backtrace)]
#![feature(try_trait)]
#![feature(type_alias_impl_trait)]

mod errors;
pub use errors::ConfigError;
mod args;
pub use args::Arguments;
pub mod commands;
pub use commands::Commands;
pub mod ui;

#[cfg(test)]
pub(crate) mod test_utils;
