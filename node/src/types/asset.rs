use super::{errors::TypeError, RaidID, TemplateID};
use bytes::{Buf, Bytes};
use postgres_protocol::types::text_from_sql;
use std::{error::Error, str::FromStr};
use tokio_postgres::types::{accepts, FromSql, ToSql, Type};

pub struct AssetID {
    template_id: TemplateID,
    features: u16,
    raid_id: RaidID,
    hash: [u8; 32],
}

impl<'a> FromSql<'a> for AssetID {
    accepts!(TEXT);

    fn from_sql(_: &Type, raw: &'a [u8]) -> Result<AssetID, Box<dyn Error + Sync + Send>> {
        Ok(text_from_sql(raw)?.parse()?)
    }
}

use anyhow::anyhow;
use std::{array::TryFromSliceError, convert::TryInto};

/// Converts AssetID from string according to rfc https://rfc.tari.com/RFC-0311_AssetTemplates.html#asset-identification
impl FromStr for AssetID {
    type Err = TypeError;

    fn from_str(hex: &str) -> Result<Self, TypeError> {
        let invalid_hex = |hex| anyhow!("AssetID is less than 64 chars '{}'", hex);
        let template_id = match hex.get(0..12) {
            None => Err(TypeError::parse_field("AssetID::template_id", invalid_hex(hex))),
            Some(buf) => TemplateID::from_hex(buf),
        }?;

        let features = match hex.get(12..16) {
            None => Err(TypeError::parse_field("AssetID::features", invalid_hex(hex))),
            Some(buf) => {
                u16::from_str_radix(buf, 16).map_err(|err| TypeError::parse_field("AssetID::features", err.into()))
            },
        }?;

        let raid_id = match hex.get(16..31) {
            None => Err(TypeError::parse_field("AssetID::raid_id", invalid_hex(hex))),
            Some(buf) => RaidID::from_base58(buf),
        }?;

        let hash = match hex.get(32..64) {
            None => Err(TypeError::parse_field("AssetID::hash", invalid_hex(hex))),
            Some(buf) => buf
                .as_bytes()
                .try_into()
                .map_err(|err: TryFromSliceError| TypeError::parse_field("AssetID::hash", err.into())),
        }?;

        Ok(Self {
            template_id,
            features,
            raid_id,
            hash,
        })
    }
}
