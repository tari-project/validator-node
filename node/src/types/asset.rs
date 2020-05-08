use super::{errors::TypeError, RaidID, TemplateID};
use bytes::BytesMut;
use postgres_protocol::types::text_from_sql;
use std::{error::Error, str::FromStr};
use tokio_postgres::types::{accepts, to_sql_checked, FromSql, IsNull, ToSql, Type};


/// Assets are identified by a 64-character string that uniquely identifies an asset on the network
/// [RFC-0311](https://rfc.tari.com/RFC-0311_AssetTemplates.html#asset-identification) entity
#[derive(Debug, Clone)]
pub struct AssetID {
    template_id: TemplateID,
    features: u16,
    raid_id: RaidID,
    hash: String,
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
impl ToString for AssetID {
    fn to_string(&self) -> String {
        format!(
            "{}{:04X}{}.{}",
            self.template_id.to_hex(),
            self.features,
            self.raid_id.to_base58(),
            self.hash
        )
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
