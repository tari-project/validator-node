use std::{error::Error, fmt};

#[derive(Debug, PartialEq)]
pub enum AuthErrorType {
    Forbidden,
    Unauthorized,
}

#[derive(Debug)]
pub struct AuthError {
    pub reason: String,
    pub error_type: AuthErrorType,
}

impl AuthError {
    pub fn new(error_type: AuthErrorType, reason: String) -> AuthError {
        AuthError { reason, error_type }
    }

    pub fn unauthorized(reason: &str) -> Self {
        Self {
            reason: reason.to_string(),
            error_type: AuthErrorType::Unauthorized,
        }
    }
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.reason)
    }
}
impl Error for AuthError {}
