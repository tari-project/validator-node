use config::ConfigError as ConfigStateError;
use std::backtrace::Backtrace;
use tari_common::{ConfigError as BootstrapConfigError, ConfigurationError};
use tari_validator_node::{db::utils::errors::DBError, template::TemplateError, wallet::WalletError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValidatorNodeError {
    #[error("DB: {0} {0}")]
    DBError(#[from] DBError, #[backtrace] Backtrace),
    #[error("Configuration issue {0}")]
    Config(#[from] ConfigError),
    #[error("Wallet error: {0}")]
    Wallet(#[from] WalletError),
    #[error("Template error: {0} {1}")]
    Template(#[from] TemplateError, #[backtrace] Backtrace),
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
