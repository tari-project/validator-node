use super::errors::TypeError;

#[derive(Debug, Clone, Copy)]
pub struct RaidID;

impl RaidID {
    pub fn from_base58(raw: &str) -> Result<Self, TypeError> {
        unimplemented!()
    }

    pub fn to_base58(&self) -> String {
        unimplemented!()
    }
}
