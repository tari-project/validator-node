use crate::types::{errors::TypeError, identity::generate_uuid_v1, NodeID};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tokio_postgres::types::{FromSql, ToSql};
use uuid::Uuid;

#[derive(Copy, Clone, PartialEq, Debug, ToSql, FromSql)]
pub struct ProposalID(pub(crate) Uuid);

impl ProposalID {
    pub async fn new(node_id: NodeID) -> Result<Self, TypeError> {
        Ok(Self(generate_uuid_v1(&node_id)?))
    }
}

impl Serialize for ProposalID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ProposalID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        Deserialize::deserialize(deserializer).map(|value| ProposalID(value))
    }
}
