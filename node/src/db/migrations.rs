use super::utils::errors::DBError;
use crate::{config::NodeConfig, db::utils::db::db_client_raw};

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!();
}

pub async fn migrate(node_config: NodeConfig) -> Result<(), DBError> {
    let mut conn = db_client_raw(&node_config).await?;
    embedded::migrations::runner().run_async(&mut conn).await?;
    Ok(())
}
