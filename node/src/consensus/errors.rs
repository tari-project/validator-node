use crate::{db::utils::errors::DBError, types::errors::TypeError};
use std::{io::Error as IOError, sync::mpsc::SendError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConsensusError {
    #[error("DB error: {0}")]
    DBError(#[from] DBError),
    #[error("Type error: {0}")]
    TypeError(#[from] TypeError),
    #[error("SendError: {0}")]
    SendError(#[from] SendError<()>),
    #[error("Issue reaching consensus: {msg}")]
    Error { msg: String },
    #[error("IO error: {0}")]
    IOError(#[from] IOError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl ConsensusError {
    pub fn error(msg: &str) -> Self {
        Self::Error { msg: msg.into() }
    }
}
