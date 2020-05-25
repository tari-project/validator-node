use crate::types::{errors::TypeError, identity::generate_uuid_v1, NodeID};
use serde::{Deserialize, Serialize};
use tokio_postgres::types::{FromSql, ToSql};
use uuid::Uuid;

#[derive(Copy, Clone, PartialEq, Debug, ToSql, FromSql, Serialize, Deserialize)]
pub struct ProposalID(pub(crate) Uuid);

impl ProposalID {
    pub async fn new(node_id: NodeID) -> Result<Self, TypeError> {
        Ok(Self(generate_uuid_v1(&node_id)?))
    }
}
