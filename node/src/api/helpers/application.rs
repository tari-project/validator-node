use crate::api::errors::*;
use actix_web::{http, http::StatusCode, HttpResponse, Responder};
use log::{error, warn};

pub fn unauthorized<T: Responder>(message: &str) -> Result<T, ApiError> {
    Err(AuthError::new(AuthErrorType::Unauthorized, message.into()).into())
}

pub fn forbidden<T: Responder>(message: &str) -> Result<T, ApiError> {
    Err(AuthError::new(AuthErrorType::Forbidden, message.into()).into())
}

pub fn unprocessable<T: Responder>(message: &str) -> Result<T, ApiError> {
    Err(ApplicationError::new_with_type(ApplicationErrorType::Unprocessable, message.to_string()).into())
}
pub fn bad_request<T: Responder>(message: &str) -> Result<T, ApiError> {
    Err(ApplicationError::new_with_type(ApplicationErrorType::BadRequest, message.to_string()).into())
}

pub fn internal_server_error<T: Responder>(message: &str) -> Result<T, ApiError> {
    error!("Internal Server Error: {}", message);
    Err(ApplicationError::new(message.to_string()).into())
}

pub fn no_content() -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::new(StatusCode::NO_CONTENT))
}

pub fn not_found() -> Result<HttpResponse, ApiError> {
    warn!("Not found");
    Ok(HttpResponse::new(StatusCode::NOT_FOUND))
}
pub fn method_not_allowed() -> Result<HttpResponse, ApiError> {
    warn!("Method not allowed");
    Ok(HttpResponse::new(StatusCode::METHOD_NOT_ALLOWED))
}

pub fn created(json: serde_json::Value) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Created().json(json))
}

pub fn redirect(url: &str) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Found().header(http::header::LOCATION, url).finish())
}
