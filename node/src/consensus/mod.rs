pub use self::{
    config::ConsensusConfig,
    consensus_committee::ConsensusCommittee,
    consensus_processor::ConsensusProcessor,
    consensus_worker::ConsensusWorker,
};

pub mod communications;
mod config;
mod consensus_committee;
mod consensus_processor;
mod consensus_worker;
pub mod errors;

const LOG_TARGET: &'static str = "tari_validator_node::consensus";
