#[derive(Debug, Clone)]
pub struct TokenID(String);

impl std::ops::Deref for TokenID {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
