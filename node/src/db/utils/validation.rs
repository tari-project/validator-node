use serde::Serialize;
use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
};
use thiserror::Error;

#[derive(Default, Debug, Serialize, Clone, PartialEq)]
pub struct ValidationError {
    pub message: String,
    pub code: String,
}

#[derive(Error, Default, Debug, Serialize, Clone, PartialEq)]
pub struct ValidationErrors(pub HashMap<&'static str, Vec<ValidationError>>);

impl ValidationErrors {
    pub fn append_validation_error(&mut self, code: &'static str, field: &'static str, message: &'static str) {
        (*self.0.entry(field).or_insert(Vec::new())).push(ValidationError {
            message: message.into(),
            code: code.into(),
        });
    }

    pub fn validate(self) -> Result<(), ValidationErrors> {
        if self.0.len() > 0 {
            return Err(self);
        }
        Ok(())
    }
}

impl Display for ValidationErrors {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;

    #[actix_rt::test]
    async fn append_validation_error() -> anyhow::Result<()> {
        let mut validation_errors = ValidationErrors::default();
        validation_errors.append_validation_error("test", "test-field", "test-message");

        let mut expected_validation_errors = HashMap::new();
        let expected_errors = vec![ValidationError {
            message: "test-message".into(),
            code: "test".into(),
        }];
        expected_validation_errors.insert("test-field", expected_errors);
        assert_eq!(validation_errors, ValidationErrors(expected_validation_errors));

        Ok(())
    }

    #[actix_rt::test]
    async fn validate() -> anyhow::Result<()> {
        let validation_errors = ValidationErrors::default();
        assert!(validation_errors.validate().is_ok());

        let mut validation_errors = ValidationErrors::default();
        validation_errors.append_validation_error("test", "test-field", "test-message");
        assert_eq!(validation_errors.clone().validate(), Err(validation_errors));

        Ok(())
    }
}
