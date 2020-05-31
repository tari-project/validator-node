use crate::types::{errors::TypeError, identity::generate_uuid_v1, NodeID};
use serde::{Deserialize, Serialize};
use std::{fmt, ops::Deref, str::FromStr};
use tokio_postgres::types::{FromSql, ToSql, Type};
use uuid::Uuid;

#[derive(Default, Copy, Clone, PartialEq, Eq, Hash, Debug, ToSql, FromSql, Deserialize, Serialize)]
pub struct InstructionID(pub(crate) Uuid);

impl InstructionID {
    pub const SQL_TYPE: Type = Type::UUID;

    pub fn new(node_id: NodeID) -> Result<Self, TypeError> {
        Ok(Self(generate_uuid_v1(&node_id)?))
    }
}

impl fmt::Display for InstructionID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:X}", self.0.to_simple())
    }
}

impl FromStr for InstructionID {
    type Err = TypeError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Ok(Self(input.parse()?))
    }
}

impl Deref for InstructionID {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
