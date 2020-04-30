use super::errors::DBError;
use crate::{config::NodeConfig, db::pool::build_pool};
use deadpool_postgres::Pool;

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!();
}

pub async fn migrate(node_config: NodeConfig) -> Result<(), DBError> {
    let pool: Pool = build_pool(&node_config.postgres)?;
    let mut conn = pool.get().await?;
    embedded::migrations::runner().run_async(&mut **conn).await?;
    Ok(())
}
