use crate::db::models::{NewAssetStateAppendOnly, NewTokenStateAppendOnly};
use bytes::BytesMut;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::error::Error;
use tokio_postgres::types::{accepts, to_sql_checked, FromSql, IsNull, Json, ToSql, Type};

#[derive(Clone, Serialize, PartialEq, Deserialize, Debug)]
pub struct AppendOnlyState {
    pub asset_state: Vec<NewAssetStateAppendOnly>,
    pub token_state: Vec<NewTokenStateAppendOnly>,
}

impl<'a> ToSql for AppendOnlyState {
    accepts!(JSON, JSONB);

    to_sql_checked!();

    fn to_sql(&self, ty: &Type, w: &mut BytesMut) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        json!(self).to_sql(ty, w)
    }
}

impl<'a> FromSql<'a> for AppendOnlyState {
    accepts!(JSON, JSONB);

    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        Ok(serde_json::from_value(
            Json::<Value>::from_sql(ty, raw).map(|json| json.0)?,
        )?)
    }
}
