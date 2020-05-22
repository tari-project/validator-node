use crate::types::{errors::TypeError, identity::generate_uuid_v1, NodeID};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tokio_postgres::types::{FromSql, ToSql};
use uuid::Uuid;

#[derive(Default, Copy, Clone, PartialEq, Debug, ToSql, FromSql)]
pub struct InstructionID(pub(crate) Uuid);

impl InstructionID {
    pub async fn new(node_id: NodeID) -> Result<Self, TypeError> {
        Ok(Self(generate_uuid_v1(&node_id)?))
    }
}

impl Serialize for InstructionID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for InstructionID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        Deserialize::deserialize(deserializer).map(InstructionID)
    }
}
