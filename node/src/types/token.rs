//! TokenID type uniquely identifies Token and it's relation to Asset
//!
//! TokenID consist of [AssetID] and [Uuid] version 1, which uniquely identifies token
//! accross all Tari network nodes.
//!
//! New TokenID's should be created via [`TokenID::new()`] supplying AssetID and node identity.

// TODO: think - should we store our IDs as base58 perhaps in database rather than our string?

use super::{errors::TypeError, AssetID, NodeID};
use bytes::BytesMut;
use postgres_protocol::types::text_from_sql;
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, error::Error, fmt, str::FromStr};
use tokio_postgres::types::{accepts, to_sql_checked, FromSql, IsNull, ToSql, Type};

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(into = "String", try_from = "String")]
pub struct TokenID {
    asset_id: AssetID,
    uid: uuid::Uuid,
}

impl FromStr for TokenID {
    type Err = TypeError;

    fn from_str(hex: &str) -> Result<Self, TypeError> {
        if hex.len() != 96 {
            return Err(TypeError::source_len("TokenID", 96, hex));
        }
        let asset_id: AssetID = hex[0..64].parse()?;
        let uid = hex[64..96].parse()?;
        Ok(Self { asset_id, uid })
    }
}

impl fmt::Display for TokenID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{:X}", self.asset_id, self.uid.to_simple())
    }
}

impl From<TokenID> for String {
    fn from(id: TokenID) -> Self {
        id.to_string()
    }
}

impl TryFrom<String> for TokenID {
    type Error = TypeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

use chrono::Local;
use uuid::{
    v1::{Context, Timestamp},
    Uuid,
};

lazy_static::lazy_static! {
    pub static ref CONTEXT: Context = Context::new(1);
}

impl TokenID {
    /// TokenID stored as BPCHAR, it might change in the future
    pub const SQL_TYPE: Type = Type::BPCHAR;

    /// Generate TokenID for AssetID on a node
    ///
    /// Cross-node uniqueness guaranteed if node_id uniquelly identifies process
    pub fn new(asset_id: &AssetID, node_id: NodeID) -> Result<Self, TypeError> {
        let time = Local::now();
        let ts = Timestamp::from_unix(&*CONTEXT, time.timestamp() as u64, time.timestamp_subsec_nanos());
        let uid = Uuid::new_v1(ts, &node_id.inner())?;
        Ok(Self {
            asset_id: asset_id.clone(),
            uid,
        })
    }

    /// Retrieve AssetID from a TokenID
    #[inline]
    pub fn asset_id(&self) -> AssetID {
        self.asset_id.clone()
    }

    #[inline]
    pub fn uid(&self) -> uuid::Uuid {
        self.uid.clone()
    }
}

impl<'a> FromSql<'a> for TokenID {
    accepts!(BPCHAR);

    fn from_sql(_: &Type, raw: &'a [u8]) -> Result<TokenID, Box<dyn Error + Sync + Send>> {
        Ok(text_from_sql(raw)?.parse()?)
    }
}

impl<'a> ToSql for TokenID {
    accepts!(BPCHAR);

    to_sql_checked!();

    fn to_sql(&self, ty: &Type, w: &mut BytesMut) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        <&str as ToSql>::to_sql(&self.to_string().as_str(), ty, w)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::utils::test_db_client;

    #[test]
    fn token_default() {
        let id = TokenID::default();
        assert_eq!(id, format!("{:031X}.{:032X}{:032X}", 0, 0, 0).parse().unwrap());
    }

    #[test]
    fn token_bad_format() {
        for bad_input in &[
            "ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ"
                .to_string(),
            "A".to_string(),
            format!("{:031X}.{:032X}{:031X}", 0, 0, 0),
            format!("{:031X}.{:032X}{:033X}", 0, 0, 0),
        ] {
            assert!(bad_input.parse::<TokenID>().is_err(), "Should fail on '{}'", bad_input)
        }
    }

    #[test]
    fn token_from_to_string() {
        let mut raw = vec!["A"; 96];
        raw[31] = ".";
        for i in 0..96 {
            if i == 31 {
                continue;
            }
            raw[i] = "1";
            let src = raw.join("");
            let id: TokenID = src.parse().expect("Failed to parse TokenID");
            let dst = id.to_string();
            assert_eq!(src, dst);
        }
    }

    #[test]
    fn new_tokens_unique() {
        let mut tokens = vec![];
        for _ in 0..100 {
            let token = TokenID::new(&AssetID::default(), [0, 1, 2, 3, 4, 5]).unwrap();
            for past in tokens.iter() {
                assert_ne!(token, *past);
            }
            tokens.push(token);
        }
    }

    #[actix_rt::test]
    async fn sql() {
        let (client, _lock) = test_db_client().await;
        let mut raw = vec!["A"; 96];
        raw[31] = ".";
        for i in 0..8 {
            raw[i * 8] = "1";
            let src = raw.join("");
            let id: TokenID = src.parse().expect("Failed to parse TokenID");
            let stmt = client.prepare_typed("SELECT $1", &[TokenID::SQL_TYPE]).await.unwrap();
            let id2: TokenID = client.query_one(&stmt, &[&id]).await.unwrap().get(0);
            assert_eq!(id, id2);
        }
    }
}
