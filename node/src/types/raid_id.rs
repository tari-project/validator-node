use super::errors::TypeError;

pub struct RaidID;

impl RaidID {
    pub fn from_base58(raw: &str) -> Result<Self, TypeError> {
        Ok(Self)
    }
}
