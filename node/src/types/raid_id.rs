use super::errors::TypeError;
use std::str::FromStr;

/// Registered Asset Issuer Domain (RAID) TXT record
/// It uniquely identifies pubkey with domain owner based on formula Hash256(PubKey || FQDN)
/// [RFC-0301](https://rfc.tari.com/RFC-0301_NamespaceRegistration.html?highlight=Raid#openalias-txt-dns-records) entity
#[derive(Debug, Hash, Eq, Clone, PartialEq)]
pub struct RaidID(String);

impl Default for RaidID {
    fn default() -> Self {
        Self("000000000000000".into())
    }
}

/// Converts RaidID from base58 15 char String
impl FromStr for RaidID {
    type Err = TypeError;

    fn from_str(input: &str) -> Result<Self, TypeError> {
        Self::from_base58(input)
    }
}

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
