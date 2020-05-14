use bytes::BytesMut;
use postgres_types::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::error::Error;
use tokio_postgres::types::{accepts, to_sql_checked, FromSql, IsNull, ToSql, Type};

#[derive(Clone, Serialize, PartialEq, Debug, Deserialize)]
pub enum NodeSelectionStrategy {
    RegisterAll = 1,
}

#[derive(Clone, Serialize, PartialEq, Debug, Deserialize)]
pub enum CommitteeMode {
    Public {
        node_threshold: u32,
        minimum_collateral: i64,
        node_selection_strategy: NodeSelectionStrategy,
    },
    Creator {
        trusted_node_set: Vec<String>,
    },
}

impl Default for CommitteeMode {
    fn default() -> CommitteeMode {
        CommitteeMode::Creator {
            trusted_node_set: vec![],
        }
    }
}

impl<'a> ToSql for CommitteeMode {
    accepts!(JSON, JSONB);

    to_sql_checked!();

    fn to_sql(&self, ty: &Type, w: &mut BytesMut) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        json!(self).to_sql(ty, w)
    }
}

impl<'a> FromSql<'a> for CommitteeMode {
    accepts!(JSON, JSONB);

    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<CommitteeMode, Box<dyn Error + Sync + Send>> {
        Ok(serde_json::from_value(
            Json::<Value>::from_sql(ty, raw).map(|json| json.0)?,
        )?)
    }
}
