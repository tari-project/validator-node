use crate::{config::NodeConfig, db::pool::build_pool};
use deadpool_postgres::Pool;

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!();
}

pub async fn migrate(node_config: NodeConfig) {
    let pool: Pool = build_pool(node_config);
    let mut conn = pool.get().await.unwrap();
    embedded::migrations::runner().run_async(&mut **conn).await.unwrap();
}
