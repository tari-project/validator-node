use thiserror::Error;

#[derive(Error, Debug)]
pub enum TemplateError {
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
