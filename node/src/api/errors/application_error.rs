use std::{error::Error, fmt};

#[derive(Debug)]
pub enum ApplicationErrorType {
    Unprocessable,
    Internal,
    BadRequest,
}

#[derive(Debug)]
pub struct ApplicationError {
    pub reason: String,
    pub error_type: ApplicationErrorType,
}

impl ApplicationError {
    pub fn new(reason: String) -> ApplicationError {
        ApplicationError {
            reason,
            error_type: ApplicationErrorType::Internal,
        }
    }

    pub fn new_with_type(error_type: ApplicationErrorType, reason: String) -> ApplicationError {
        ApplicationError { reason, error_type }
    }

    pub fn unprocessable(reason: &str) -> ApplicationError {
        ApplicationError::new_with_type(ApplicationErrorType::Unprocessable, reason.to_string())
    }

    pub fn bad_request(reason: &str) -> Self {
        Self::new_with_type(ApplicationErrorType::BadRequest, reason.to_string())
    }
}

impl fmt::Display for ApplicationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.reason)
    }
}
impl Error for ApplicationError {}
