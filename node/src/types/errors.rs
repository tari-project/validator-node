use thiserror::Error;

#[derive(Error, Debug)]
pub enum TypeError {
    #[error("Failed to parse {field}: {source}")]
    ParseField {
        field: &'static str,
        #[source]
        source: anyhow::Error,
    },
    #[error("Failed to parse {field} from source string {raw}")]
    ParseFieldRaw { field: &'static str, raw: String },
    #[error("{obj} should be {len}-char string, got {raw} instead")]
    SourceLen { obj: &'static str, len: usize, raw: String },
    #[error("Failed to generate uuid {0}")]
    Uuid(#[from] uuid::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl TypeError {
    pub(crate) fn parse_field(field: &'static str, source: anyhow::Error) -> Self {
        Self::ParseField { field, source }
    }

    pub(crate) fn parse_field_raw(field: &'static str, raw: &str) -> Self {
        Self::ParseFieldRaw {
            field,
            raw: raw.to_owned(),
        }
    }

    pub(crate) fn source_len(obj: &'static str, len: usize, raw: &str) -> Self {
        Self::SourceLen {
            obj,
            len,
            raw: raw.to_owned(),
        }
    }
}

use actix_web::{http::StatusCode, HttpResponse, ResponseError};

impl ResponseError for TypeError {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }

    fn error_response(&self) -> HttpResponse {
        log::debug!("TypeError: {}", self.to_string());
        HttpResponse::BadRequest().json(serde_json::json!({"error": self.to_string()}))
    }
}
