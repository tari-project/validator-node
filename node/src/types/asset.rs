use super::{errors::TypeError, RaidID, TemplateID};
use bytes::BytesMut;
use postgres_protocol::types::text_from_sql;
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, error::Error, fmt, str::FromStr};
use tokio_postgres::types::{accepts, to_sql_checked, FromSql, IsNull, ToSql, Type};

/// Assets are identified by a 64-character string that uniquely identifies an asset on the network
/// [RFC-0311](https://rfc.tari.com/RFC-0311_AssetTemplates.html#asset-identification) entity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct AssetID {
    template_id: TemplateID,
    features: u16,
    raid_id: RaidID,
    hash: String,
}

impl Default for AssetID {
    fn default() -> Self {
        Self {
            template_id: 0.into(),
            features: 0,
            raid_id: RaidID::from_base58("000000000000000").unwrap(),
            hash: format!("{:032X}", 0),
        }
    }
}

impl AssetID {
    /// AssetID stored as TEXT
    pub const SQL_TYPE: Type = Type::TEXT;

    pub fn template_id(&self) -> TemplateID {
        self.template_id.clone()
    }
}

impl<'a> FromSql<'a> for AssetID {
    accepts!(TEXT);

    fn from_sql(_: &Type, raw: &'a [u8]) -> Result<AssetID, Box<dyn Error + Sync + Send>> {
        Ok(text_from_sql(raw)?.parse()?)
    }
}

impl<'a> ToSql for AssetID {
    accepts!(TEXT);

    to_sql_checked!();

    fn to_sql(&self, ty: &Type, w: &mut BytesMut) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        <&str as ToSql>::to_sql(&self.to_string().as_str(), ty, w)
    }
}

/// Converts AssetID to string according to rfc https://rfc.tari.com/RFC-0311_AssetTemplates.html#asset-identification
impl fmt::Display for AssetID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{:04X}{}.{}",
            self.template_id.to_hex(),
            self.features,
            self.raid_id.to_base58(),
            self.hash
        )
    }
}

impl From<AssetID> for String {
    fn from(id: AssetID) -> Self {
        id.to_string()
    }
}

/// Converts AssetID from string according to rfc https://rfc.tari.com/RFC-0311_AssetTemplates.html#asset-identification
impl FromStr for AssetID {
    type Err = TypeError;

    fn from_str(hex: &str) -> Result<Self, TypeError> {
        if hex.len() != 64 {
            return Err(TypeError::source_len("AssetID", 64, hex));
        }

        let template_id = match hex.get(0..12) {
            None => Err(TypeError::parse_field_raw("AssetID::template_id", hex)),
            Some(buf) => TemplateID::from_hex(buf),
        }?;

        let features = match hex.get(12..16) {
            None => Err(TypeError::parse_field_raw("AssetID::features", hex)),
            Some(buf) => {
                u16::from_str_radix(buf, 16).map_err(|err| TypeError::parse_field("AssetID::features", err.into()))
            },
        }?;

        let raid_id = match hex.get(16..31) {
            None => Err(TypeError::parse_field_raw("AssetID::raid_id", hex)),
            Some(buf) => RaidID::from_base58(buf),
        }?;

        let hash = match hex.get(32..64) {
            None => Err(TypeError::parse_field_raw("AssetID::hash", hex))?,
            Some(buf) => buf.to_string(),
        };

        Ok(Self {
            template_id,
            features,
            raid_id,
            hash,
        })
    }
}

impl TryFrom<String> for AssetID {
    type Error = TypeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::test_db_client;

    #[test]
    fn asset_default() {
        let id = AssetID::default();
        assert_eq!(id, format!("{:031X}.{:032X}", 0, 0).parse().unwrap());
    }

    #[test]
    fn asset_from_to_string() {
        let mut raw = vec!["A"; 64];
        raw[31] = ".";
        for i in 0..64 {
            if i == 31 {
                continue;
            }
            raw[i] = "1";
            let src = raw.join("");
            let id: AssetID = src.parse().expect("Failed to parse AssetID");
            let dst = id.to_string();
            assert_eq!(src, dst);
        }
    }

    #[actix_rt::test]
    async fn sql() -> anyhow::Result<()> {
        dotenv::dotenv().unwrap();
        let (client, _lock) = test_db_client().await;
        let mut raw = vec!["A"; 64];
        raw[31] = ".";
        for i in 0..8 {
            raw[i * 8] = "1";
            let src = raw.join("");
            let id: AssetID = src.parse().expect("Failed to parse AssetID");
            let stmt = client.prepare_typed("SELECT $1", &[AssetID::SQL_TYPE]).await?;
            let id2: AssetID = client.query_one(&stmt, &[&id]).await?.get(0);
            assert_eq!(id, id2);
        }
        Ok(())
    }
}
