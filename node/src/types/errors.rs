use thiserror::Error;

#[derive(Error, Debug)]
pub enum TypeError {
    #[error("Failed to parse {field}: {source}")]
    ParseFieldError {
        field: &'static str,
        #[source]
        source: anyhow::Error,
    },
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

impl TypeError {
    pub(crate) fn parse_field(field: &'static str, source: anyhow::Error) -> Self {
        Self::ParseFieldError { field, source }
    }
}
