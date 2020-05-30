use crate::db::utils::errors::DBError;
use thiserror::Error;

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
