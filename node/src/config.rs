use serde::{Deserialize, Serialize};
use tari_common::NetworkConfigPath;
use super::server::ActixConfig;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct NodeConfig {
    pub actix: ActixConfig,
    /// see [deadpool_postgres::config::Config] on env + config vars details
    #[serde(skip_serializing)]
    pub postgres: deadpool_postgres::config::Config,
}

impl NetworkConfigPath for NodeConfig {
    fn main_key_prefix() -> &'static str {
        "bigneon_node"
    }
}
