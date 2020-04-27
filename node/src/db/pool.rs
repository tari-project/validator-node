use crate::config::NodeConfig;
use deadpool_postgres::Pool;
use tokio_postgres::NoTls;

pub fn build_pool(node_config: NodeConfig) -> Pool {
    node_config.postgres.create_pool(NoTls).unwrap()
}
