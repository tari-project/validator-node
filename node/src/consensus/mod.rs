pub use self::{config::ConsensusConfig, consensus_committee::ConsensusCommittee};

pub mod communications;
mod config;
pub mod consensus_committee;
pub mod errors;
pub mod processor;

const LOG_TARGET: &'static str = "consensus";
