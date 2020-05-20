use crate::db::utils::errors::DBError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConsensusError {
    #[error("DB error: {0}")]
    DBError(#[from] DBError),
    #[error("Issue reaching consensus: {msg}")]
    Error { msg: String },
    // TODO: Flesh out the consensus errors further
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl ConsensusError {
    pub fn error(msg: &str) -> Self {
        Self::Error { msg: msg.into() }
    }
}
