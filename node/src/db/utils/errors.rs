use crate::db::utils::validation::ValidationErrors;
use deadpool_postgres::{config::ConfigError as PoolConfigError, PoolError};
use refinery::Error as MigrationsError;
use tari_crypto::tari_utilities::hex::HexError;
use thiserror::Error;
use tokio_pg_mapper::Error as PGMError;
use tokio_postgres::error::Error as PgError;

#[derive(Error, Debug)]
pub enum DBError {
    #[error("DB pool error: {0}")]
    Pool(#[from] PoolError),
    #[error("DB pool configuration error: {0}")]
    PoolConfig(#[from] PoolConfigError),
    #[error("Postgres error: {0}")]
    Postgres(#[from] PgError),
    #[error("Postgres data mapping error: {0:?}")]
    PostgresMapping(#[from] PGMError),
    #[error("{0}")]
    HexError(#[from] HexError),
    #[error("DB migrations error: {0}")]
    Migration(#[from] MigrationsError),
    #[error("Bad query: {msg}")]
    BadQuery { msg: String },
    #[error("Not found")]
    NotFound,
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationErrors),
}

impl DBError {
    pub fn bad_query(msg: &str) -> Self {
        Self::BadQuery { msg: msg.into() }
    }
}
