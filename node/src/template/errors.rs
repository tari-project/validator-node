use crate::{db::utils::errors::DBError, errors::WalletError};
use std::backtrace::Backtrace;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TemplateError {
    #[error("DB error in Template: {source:?}")]
    DB {
        #[from]
        source: DBError,
        backtrace: Backtrace,
    },
    #[error("Wallet error in Template: {source}")]
    Wallet {
        #[from]
        source: WalletError,
        backtrace: Backtrace,
    },
    #[error("Template processing failed: {0}")]
    Processing(String),
    #[error("Contract parameters validation failed: {0}")]
    Validation(#[from] anyhow::Error),
    #[error("Failed to send message {params} to actor {name}: {source}")]
    ActorSend {
        params: String,
        name: String,
        #[source]
        source: anyhow::Error,
    },
    #[error("Failed to receive actor response: {source}")]
    ActorResponse{
        #[from]
        source: actix::MailboxError,
        backtrace: Backtrace,
    },
    #[error("Internal Template error: {0}")]
    Internal(#[source] anyhow::Error),
}

#[macro_export]
macro_rules! internal_err {
    ($msg:literal $(,)?) => {
        Err(TemplateError::Internal(anyhow::anyhow!($msg)))
    };
    ($fmt:expr, $($arg:tt)*) => {
        Err(TemplateError::Internal(anyhow::anyhow!($fmt, $($arg)*)))
    };
}

#[macro_export]
macro_rules! processing_err {
    ($msg:literal $(,)?) => {
        Err(TemplateError::Processing($msg.into()))
    };
    ($fmt:expr, $($arg:tt)*) => {
        Err(TemplateError::Processing(format!($fmt, $($arg)*)))
    };
}

#[macro_export]
macro_rules! validation_err {
    ($msg:literal $(,)?) => {
        Err(TemplateError::Validation(anyhow::anyhow!($msg)))
    };
    ($fmt:expr, $($arg:tt)*) => {
        Err(TemplateError::Validation(anyhow::anyhow!($fmt, $($arg)*)))
    };
}
