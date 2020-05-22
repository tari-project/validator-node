use super::*;
use crate::{db::utils::errors::DBError, template::errors::TemplateError, types::errors::TypeError};
use actix_web::{error::ResponseError, http::StatusCode, HttpResponse};
use serde_json::json;
use std::backtrace::Backtrace;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("DB error: {source}: {backtrace:?}")]
    DBError {
        #[from]
        source: DBError,
        backtrace: Backtrace,
    },
    #[error("Incorrect value: {0}")]
    Type(#[from] TypeError),
    #[error("Application error: {source}: {backtrace:?}")]
    ApplicationError {
        #[from]
        source: ApplicationError,
        backtrace: Backtrace,
    },
    #[error("Auth error: {0}")]
    AuthError(#[from] AuthError),
    #[error("Template error: {source}: {backtrace:?}")]
    Template {
        #[from]
        source: TemplateError,
        backtrace: Backtrace,
    },
}

pub struct ResponseData {
    pub status_code: StatusCode,
    pub error_response: HttpResponse,
}

// TODO: move this to individual modules, impl ResponseError to DBError and TemplateError
impl ApiError {
    pub fn load_response_data(&self) -> ResponseData {
        let generic_error_response_data = ResponseData {
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            error_response: HttpResponse::InternalServerError().json(json!({"error": "An error has occurred"})),
        };
        match self {
            ApiError::ApplicationError{ source: ApplicationError {
                error_type, ..
            }, ..} => {
                match error_type {
                    ApplicationErrorType::Unprocessable => ResponseData {
                        status_code: StatusCode::UNPROCESSABLE_ENTITY,
                        error_response: HttpResponse::UnprocessableEntity()
                            .json(json!({"error": "Application failed to process request"})),
                    },
                    ApplicationErrorType::Internal => ResponseData {
                        status_code: StatusCode::INTERNAL_SERVER_ERROR,
                        error_response: HttpResponse::InternalServerError()
                            .json(json!({"error": "An internal error has occurred."})),
                    },
                    ApplicationErrorType::BadRequest => ResponseData {
                        status_code: StatusCode::BAD_REQUEST,
                        error_response: HttpResponse::BadRequest()
                            .json(json!({"error": "An error has occurred processing your request, please check your input and try again."})),
                    },
                }
            },
            ApiError::AuthError(AuthError { reason: _, error_type }) => {
                if *error_type == AuthErrorType::Forbidden {
                    ResponseData {
                        status_code: StatusCode::FORBIDDEN,
                        error_response: HttpResponse::build(StatusCode::FORBIDDEN)
                            .json(json!({"error": "Forbidden".to_string()})),
                    }
                } else {
                    ResponseData {
                        status_code: StatusCode::UNAUTHORIZED,
                        error_response: HttpResponse::build(StatusCode::UNAUTHORIZED)
                            .json(json!({"error": "Unauthorized".to_string()})),
                    }
                }
            },
            ApiError::DBError{source, ..} |
            ApiError::Template{source: TemplateError::DB { source, .. }, .. }
            => match source {
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
                _ => generic_error_response_data,
            },
            ApiError::Type(err) => ResponseData {
                status_code: StatusCode::BAD_REQUEST,
                error_response: HttpResponse::build(StatusCode::BAD_REQUEST)
                    .json(json!({ "error": err.to_string() })),
            },
            ApiError::Template{source: TemplateError::Validation(err), .. } => ResponseData {
                status_code: StatusCode::BAD_REQUEST,
                error_response: HttpResponse::build(StatusCode::BAD_REQUEST)
                    .json(json!({ "error": err.to_string() })),
            },
            ApiError::Template{ source, .. } => ResponseData {
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
                error_response: HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
                    .json(json!({ "error": source.to_string() })),
            },
        }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        self.load_response_data().status_code
    }

    fn error_response(&self) -> HttpResponse {
        let response_data = self.load_response_data();
        if response_data.status_code.is_server_error() {
            log::error!(target: LOG_TARGET, "Server error: {}", self);
        } else if response_data.status_code.is_client_error() {
            log::info!(target: LOG_TARGET, "Client error: {}", self);
        }

        response_data.error_response
    }
}
