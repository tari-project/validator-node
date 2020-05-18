#![deny(unreachable_patterns)]
#![deny(unknown_lints)]
#![cfg_attr(not(debug_assertions), deny(unused_variables))]
#![cfg_attr(not(debug_assertions), deny(unused_imports))]
#![cfg_attr(not(debug_assertions), deny(dead_code))]
#![deny(unused_must_use)]
#![cfg_attr(not(debug_assertions), deny(unused_extern_crates))]

pub mod api;
pub mod cli;
pub mod config;
pub mod db;
pub mod errors;
pub mod template;
pub mod types;
pub mod wallet;

#[cfg(test)]
pub mod test_utils;
