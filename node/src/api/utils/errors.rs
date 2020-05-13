use crate::db::utils::errors::DBError;
use actix_web::{error::ResponseError, http::StatusCode, HttpResponse};
use log::error;
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("DB error: {0}")]
    DBError(#[from] DBError),
    #[error("Bad request: {0}")]
    BadRequest(String),
}

pub struct ResponseData {
    pub status_code: StatusCode,
    pub error_response: HttpResponse,
}

impl ApiError {
    pub fn load_response_data(&self) -> ResponseData {
        let generic_error_response_data = ResponseData {
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            error_response: HttpResponse::InternalServerError().json(json!({"error": "An error has occurred"})),
        };
        match self {
            ApiError::DBError(db_error) => match db_error {
                DBError::Pool(_) => generic_error_response_data,
                DBError::PoolConfig(_) => generic_error_response_data,
                DBError::Postgres(postgres_error) => {
                    if let Some(code) = postgres_error.code() {
                        let (status_code, message) = match code.code() {
                            "01000" => (StatusCode::BAD_REQUEST, "Invalid input"),
                            "02000" => (StatusCode::NOT_FOUND, "No results"),
                            "23505" => (StatusCode::CONFLICT, "Duplicate record exists"),
                            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Unknown error"),
                        };

                        let error_response =
                            HttpResponse::build(status_code).json(json!({"error": message.to_string()}));
                        ResponseData {
                            status_code,
                            error_response,
                        }
                    } else {
                        generic_error_response_data
                    }
                },
                DBError::PostgresMapping(_) => generic_error_response_data,
                DBError::HexError(_) => generic_error_response_data,
                DBError::Migration(_) => generic_error_response_data,
                DBError::BadQuery { msg: _ } => generic_error_response_data,
                DBError::NotFound => ResponseData {
                    status_code: StatusCode::NOT_FOUND,
                    error_response: HttpResponse::build(StatusCode::NOT_FOUND)
                        .json(json!({"error": "No results".to_string()})),
                },
                DBError::Validation(validation_errors) => ResponseData {
                    status_code: StatusCode::UNPROCESSABLE_ENTITY,
                    error_response: HttpResponse::UnprocessableEntity()
                        .json(json!({"error": "Validation error".to_string(), "fields": validation_errors})),
                },
            },
            ApiError::BadRequest(msg) => ResponseData {
                status_code: StatusCode::BAD_REQUEST,
                error_response: HttpResponse::build(StatusCode::BAD_REQUEST).json(json!({ "error": msg })),
            },
        }
    }
}

impl ApiError {
    pub fn bad_request(msg: &str) -> Self {
        Self::BadRequest(msg.into())
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        self.load_response_data().status_code
    }

    fn error_response(&self) -> HttpResponse {
        let response_data = self.load_response_data();
        if response_data.status_code != StatusCode::NOT_FOUND {
            error!("{:?}", self);
        }

        response_data.error_response
    }
}
