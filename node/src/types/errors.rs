use std::num::ParseIntError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TypeError {
    #[error("{message}")]
    ParseError {
        message: String,
        #[source]
        source: anyhow::Error,
    },
    #[error("Failed to parse {field}: {source}")]
    ParseFieldError {
        field: &'static str,
        #[source]
        source: ParseIntError,
    },
}

impl TypeError {
    pub(crate) fn parse(message: String) -> Self {
        Self::ParseError { message }
    }

    pub(crate) fn parse_field(field: &'static str, source: anyhow::Error) -> Self {
        Self::ParseFieldError { field, source }
    }
}
