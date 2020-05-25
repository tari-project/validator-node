use crate::types::{errors::TypeError, identity::generate_uuid_v1, NodeID};
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use tokio_postgres::types::{FromSql, ToSql};
use uuid::Uuid;

#[derive(Default, Copy, Clone, PartialEq, Debug, ToSql, FromSql, Deserialize, Serialize)]
pub struct InstructionID(pub(crate) Uuid);

impl InstructionID {
    pub async fn new(node_id: NodeID) -> Result<Self, TypeError> {
        Ok(Self(generate_uuid_v1(&node_id)?))
    }
}

impl Deref for InstructionID {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
