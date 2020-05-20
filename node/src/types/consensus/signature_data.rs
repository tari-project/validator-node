use crate::types::NodeID;
use bytes::BytesMut;
use postgres_types::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, error::Error};
use tokio_postgres::types::{accepts, to_sql_checked, FromSql, IsNull, ToSql, Type};

#[derive(Clone, Serialize, PartialEq, Debug, Deserialize)]
pub struct SignatureData {
    pub signatures: HashMap<NodeID, String>,
}

impl Default for SignatureData {
    fn default() -> SignatureData {
        SignatureData {
            signatures: HashMap::new(),
        }
    }
}

impl<'a> ToSql for SignatureData {
    accepts!(JSON, JSONB);

    to_sql_checked!();

    fn to_sql(&self, ty: &Type, w: &mut BytesMut) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        json!(self).to_sql(ty, w)
    }
}

impl<'a> FromSql<'a> for SignatureData {
    accepts!(JSON, JSONB);

    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<SignatureData, Box<dyn Error + Sync + Send>> {
        Ok(serde_json::from_value(
            Json::<Value>::from_sql(ty, raw).map(|json| json.0)?,
        )?)
    }
}
