use super::errors::TypeError;

#[derive(Debug, Clone)]
pub struct RaidID(String);

impl RaidID {
    pub fn from_base58(raw: &str) -> Result<Self, TypeError> {
        if raw.len() != 15 {
            return Err(TypeError::source_len("RaidID", 12, raw));
        }
        Ok(Self(raw.to_owned()))
    }

    pub fn to_base58(&self) -> String {
        self.0.clone()
    }
}


