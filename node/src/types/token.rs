//! Stub
use super::{errors::TypeError, AssetID};
use bytes::BytesMut;
use postgres_protocol::types::text_from_sql;
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt, ops::Deref, str::FromStr};
use tokio_postgres::types::{accepts, to_sql_checked, FromSql, IsNull, ToSql, Type};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TokenID(String);

impl FromStr for TokenID {
    type Err = TypeError;

    fn from_str(hex: &str) -> Result<Self, TypeError> {
        if hex.len() != 96 {
            return Err(TypeError::source_len("TokenID", 96, hex));
        }
        Ok(Self(hex.into()))
    }
}

impl fmt::Display for TokenID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TokenID {
    pub fn asset_id(&self) -> Result<AssetID, TypeError> {
        Ok(self.0[0..64].parse()?)
    }
}

impl Deref for TokenID {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> FromSql<'a> for TokenID {
    accepts!(TEXT);

    fn from_sql(_: &Type, raw: &'a [u8]) -> Result<TokenID, Box<dyn Error + Sync + Send>> {
        Ok(text_from_sql(raw)?.parse()?)
    }
}

impl<'a> ToSql for TokenID {
    accepts!(TEXT);

    to_sql_checked!();

    fn to_sql(&self, ty: &Type, w: &mut BytesMut) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        <&str as ToSql>::to_sql(&self.to_string().as_str(), ty, w)
    }
}
