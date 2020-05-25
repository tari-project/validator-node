use super::errors::TypeError;
use crate::types::node_id::NodeID;
use chrono::Local;
use uuid::{
    v1::{Context, Timestamp},
    Uuid,
};

lazy_static::lazy_static! {
    pub static ref CONTEXT: Context = Context::new(1);
}

pub fn generate_uuid_v1(node_id: &NodeID) -> Result<Uuid, TypeError> {
    let time = Local::now();
    let ts = Timestamp::from_unix(&*CONTEXT, time.timestamp() as u64, time.timestamp_subsec_nanos());
    Ok(Uuid::new_v1(ts, &node_id.inner())?)
}
