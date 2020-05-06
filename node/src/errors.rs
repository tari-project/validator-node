use crate::db::errors::DBError;
use config::ConfigError as ConfigStateError;
use tari_common::{ConfigError as BootstrapConfigError, ConfigurationError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValidatorNodeError {
    #[error("DB: {0}")]
    DBError(#[from] DBError),
    #[error("{0}")]
    Config(#[from] ConfigError),
    #[error("Wallet error: {0}")]
    Wallet(#[from] WalletError),
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Bootstrapping error: {0}")]
    Bootstrap(#[from] BootstrapConfigError),
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigurationError),
    #[error("IO configuration error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Configuration error: {0}")]
    Source(#[from] ConfigStateError),
}

/// Errors during Wallet operations
#[derive(Error, Debug)]
pub enum WalletError {
    #[error("Node Identity failure: {0}")]
    NodeIdentity(#[from] tari_comms::peer_manager::NodeIdentityError),
    #[error("FS error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Json parsing error: {0}")]
    JSON(#[from] serde_json::Error),
    #[error("Wallet not found: {pubkey}")]
    NotFound { pubkey: String },
    #[error("DB error: {0}")]
    DBError(#[from] DBError),
}
impl WalletError {
    pub(crate) fn not_found(pubkey: String) -> Self {
        Self::NotFound { pubkey }
    }
}
