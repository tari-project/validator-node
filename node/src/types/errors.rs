use thiserror::Error;

#[derive(Error, Debug)]
pub enum TypeError {
    #[error("Parse error: {message}")]
    Parse {
        message: String,
    },
    #[error("Failed to parse {field}: {source}")]
    ParseFieldError {
        field: &'static str,
        #[source]
        source: anyhow::Error,
    },
}

impl TypeError {
    pub(crate) fn parse(message: String) -> Self {
        Self::Parse { message  }
    }

    pub(crate) fn parse_field(field: &'static str, source: anyhow::Error) -> Self {
        Self::ParseFieldError { field, source }
    }
}
