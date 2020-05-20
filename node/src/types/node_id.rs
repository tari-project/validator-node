//! Stub
use bytes::BytesMut;
use serde::{Deserialize, Serialize};
use std::{convert::TryInto, error::Error};
use tokio_postgres::types::{accepts, to_sql_checked, FromSql, IsNull, ToSql, Type};

#[derive(Serialize, Hash, Eq, Deserialize, Default, Debug, Clone, Copy, PartialEq)]
pub struct NodeID([u8; 6]);

impl NodeID {
    pub fn inner(&self) -> [u8; 6] {
        self.0
    }

    #[doc(hidden)]
    pub(crate) fn stub() -> Self {
        Self([0, 1, 2, 3, 4, 5])
    }
}

impl<'a> FromSql<'a> for NodeID {
    accepts!(BYTEA);

    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<NodeID, Box<dyn Error + Sync + Send>> {
        Ok(NodeID(<&[u8] as FromSql>::from_sql(ty, raw).try_into()?))
    }
}

impl<'a> ToSql for NodeID {
    accepts!(BYTEA);

    to_sql_checked!();

    fn to_sql(&self, ty: &Type, w: &mut BytesMut) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        <&[u8] as ToSql>::to_sql(&*self.inner(), ty, w)
    }
}
