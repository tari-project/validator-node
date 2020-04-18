use serde::{Deserialize, Serialize, Serializer, ser::SerializeMap};
use tari_common::NetworkConfigPath;
use super::server::ActixConfig;
use deadpool_postgres::config::Config as DeadpoolConfig;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct NodeConfig {
    pub actix: ActixConfig,
    /// see [deadpool_postgres::config::Config] on env + config vars details
    #[serde(serialize_with = "default_postgres_config")]
    pub postgres: DeadpoolConfig,
}

impl NetworkConfigPath for NodeConfig {
    fn main_key_prefix() -> &'static str {
        "validator"
    }
}

// Database default parameters
fn default_postgres_config<S: Serializer>(_: &DeadpoolConfig, s: S) -> Result<S::Ok, S::Error> {
    let mut db = s.serialize_map(None)?;
    db.serialize_entry("dbname", "validator")?;
    db.end()
}
