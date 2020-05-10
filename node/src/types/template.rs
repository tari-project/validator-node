use super::errors::TypeError;
use bytes::BytesMut;
use core::cmp::PartialEq;
use postgres_protocol::types::int8_from_sql;
use std::{
    convert::TryInto,
    error::Error,
    fmt,
    hash::{Hash, Hasher},
};
use tokio_postgres::types::{accepts, to_sql_checked, FromSql, IsNull, ToSql, Type};

const BETA_MASK: u16 = 1;
const CONFIDENTIAL_MASK: u16 = 2;

/// Tari uses templates to define the behaviour for its smart contracts.
/// TemplateID identifies the type of digital asset being created and smart contracts available.
/// [RFC-0311](https://rfc.tari.com/RFC-0311_AssetTemplates.html#template-id) entity
#[derive(Debug, Clone, Copy)]
pub struct TemplateID {
    template_type: u32,
    template_version: u16,
    tail: u16,
}
impl fmt::Display for TemplateID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.template_type, self.template_version)?;
        if self.beta() {
            write!(f, "-beta")?;
        }
        if self.confidential() {
            write!(f, "-confidential")?;
        }
        Ok(())
    }
}

/// Only template type and template version take part in comparison
impl PartialEq for TemplateID {
    fn eq(&self, other: &Self) -> bool {
        self.template_type == other.template_type && self.template_version == other.template_version
    }
}

/// Only template type and template version take part in hashing
impl Hash for TemplateID {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.template_type.hash(state);
        self.template_version.hash(state);
    }
}

impl TemplateID {
    /// Template type (0 - 4,294,967,295)
    #[inline]
    pub fn template_type(&self) -> u32 {
        self.template_type
    }

    /// Template version (0 - 65,535)
    #[inline]
    pub fn template_version(&self) -> u16 {
        self.template_version
    }

    /// Beta Mode flag
    #[inline]
    pub fn beta(&self) -> bool {
        self.tail & BETA_MASK != 0
    }

    /// Confidentiality flag
    #[inline]
    pub fn confidential(&self) -> bool {
        self.tail & CONFIDENTIAL_MASK != 0
    }

    /// Template type as 8-byte hex
    #[inline]
    pub fn type_hex(&self) -> String {
        format!("{:08X}", self.template_type)
    }

    /// Template version as 4-byte hex
    #[inline]
    pub fn version_hex(&self) -> String {
        format!("{:04X}", self.template_version)
    }

    /// Convert to 12-char hex, losing beta and confidential flag info
    #[inline]
    pub fn to_hex(&self) -> String {
        format!("{:08X}{:04X}", self.template_type, self.template_version)
    }

    /// Convert from 12-char hex, considering beta and confidential is false
    pub fn from_hex(hex: &str) -> Result<Self, TypeError> {
        if hex.len() != 12 {
            return Err(TypeError::source_len("TemplateID", 12, hex));
        }

        let template_type = u32::from_str_radix(&hex[0..8], 16)
            .map_err(|err| TypeError::parse_field("TemplateID::type", err.into()))?;
        let template_version = u16::from_str_radix(&hex[8..12], 16)
            .map_err(|err| TypeError::parse_field("TemplateID::version", err.into()))?;
        Ok(Self {
            template_type,
            template_version,
            tail: 0,
        })
    }
}

/// Load TemplateID from 64-bit unsigned int
/// See https://rfc.tari.com/RFC-0311_AssetTemplates.html#template-id
impl From<u64> for TemplateID {
    fn from(src: u64) -> Self {
        let buf = src.to_le_bytes();
        Self {
            template_type: u32::from_le_bytes(buf[0..4].try_into().unwrap()),
            template_version: u16::from_le_bytes(buf[4..6].try_into().unwrap()),
            tail: u16::from_le_bytes(buf[6..8].try_into().unwrap()),
        }
    }
}

/// TemplateID is usually stored as 64-bit unsigned int
/// See https://rfc.tari.com/RFC-0311_AssetTemplates.html#template-id
impl From<&TemplateID> for u64 {
    fn from(id: &TemplateID) -> u64 {
        let mut dst: [u8; 8] = [0; 8];
        dst[0..4].copy_from_slice(&id.template_type.to_le_bytes());
        dst[4..6].copy_from_slice(&id.template_version.to_le_bytes());
        dst[6..8].copy_from_slice(&id.tail.to_le_bytes());
        u64::from_le_bytes(dst)
    }
}

impl<'a> FromSql<'a> for TemplateID {
    accepts!(INT8);

    fn from_sql(_: &Type, raw: &'a [u8]) -> Result<TemplateID, Box<dyn Error + Sync + Send>> {
        let i64_le = int8_from_sql(raw)?.to_le_bytes();
        Ok(u64::from_le_bytes(i64_le).into())
    }
}

impl<'a> ToSql for TemplateID {
    accepts!(INT8);

    to_sql_checked!();

    fn to_sql(&self, ty: &Type, w: &mut BytesMut) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        let u64_le = u64::from(self).to_le_bytes();
        <i64 as ToSql>::to_sql(&i64::from_le_bytes(u64_le), ty, w)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::{load_env, test_db_client};

    const BETA_MASK_TEST: u64 = 1 << 48;
    const CONFIDENTIAL_MASK_TEST: u64 = 1 << 49;

    #[test]
    fn template_from_u64() {
        let sut: TemplateID = 1.into();
        assert_eq!(sut.template_type, 1);
        assert_eq!(sut.template_version, 0);
        assert!(!sut.beta(), "Expected not beta");
        assert!(!sut.confidential(), "Expected not confidential");

        let sut: TemplateID = (2 | 1 << 32).into();
        assert_eq!(sut.template_type, 2);
        assert_eq!(sut.template_version, 1);
        assert!(!sut.beta(), "Expected not beta");
        assert!(!sut.confidential(), "Expected not confidential");

        let src = 9 | 1 << 33 | BETA_MASK_TEST | CONFIDENTIAL_MASK_TEST;
        let sut: TemplateID = src.into();
        assert_eq!(sut.template_type, 9);
        assert_eq!(sut.template_version, 2, "source: {:X}", src);
        assert!(sut.beta(), "Expected not beta");
        assert!(sut.confidential(), "Expected not confidential");
        assert_eq!(sut.tail, BETA_MASK | CONFIDENTIAL_MASK, "source: {:X}", src);

        let src = 65535 | 1 << 50 | BETA_MASK_TEST | CONFIDENTIAL_MASK_TEST;
        let sut: TemplateID = src.into();
        assert_eq!(sut.template_type, 65535);
        assert_eq!(sut.template_version, 0, "source: {:X}", src);
        assert!(sut.beta(), "Expected not beta");
        assert!(sut.confidential(), "Expected not confidential");
        assert_eq!(sut.tail, 1 << 2 | BETA_MASK | CONFIDENTIAL_MASK, "source: {:X}", src);
    }

    #[test]
    fn template_to_u64() {
        for shift in 0..64 {
            let num: u64 = 1 | 7 << shift;
            let id: TemplateID = num.into();
            assert_eq!(num, u64::from(&id), "Failed conversion from {:X} to {:?}", num, id);
        }
    }

    #[test]
    fn template_from_to_hex() {
        for shift in 0..48 {
            let num: u64 = 1 | 65535 << shift;
            let mut hex = format!("{:012X}", num);
            hex.truncate(12);
            let id = TemplateID::from_hex(hex.as_str()).expect("Failed to convert hex string to TemplateID");
            assert_eq!(
                hex,
                id.to_hex(),
                "TemplateID from_hex and to_hex does not match {:?}",
                id
            );
        }
    }

    #[actix_rt::test]
    async fn sql() -> anyhow::Result<()> {
        load_env();
        let (client, _lock) = test_db_client().await;
        for shift in 0u8..15 {
            let num: u64 = 1 | (7 << (shift * 4));
            let id: TemplateID = num.into();
            let stmt = client.prepare_typed("SELECT $1", &[Type::INT8]).await?;
            let id2: TemplateID = client.query_one(&stmt, &[&id]).await?.get(0);
            assert_eq!(id, id2);
        }
        Ok(())
    }
}
