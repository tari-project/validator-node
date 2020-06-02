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

// TODO: think of moving api to separate crate
pub mod api;
// TODO: think of moving config to separate crate
pub mod config;
pub mod consensus;
pub mod db;
pub mod metrics;
pub mod template;
pub mod types;
pub mod wallet;

#[cfg(test)]
pub(crate) mod test;
