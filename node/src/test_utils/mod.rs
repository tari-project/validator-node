use crate::{config::NodeConfig, db::migrations::migrate};
use config::Source;
use deadpool_postgres::{config::Config as DeadpoolConfig, Pool};
use tokio_postgres::NoTls;

pub(crate) mod builders;

/// Generate a standard test config
pub fn build_test_config() -> anyhow::Result<NodeConfig> {
    let mut config = config::Config::new();
    let pg = config::Environment::with_prefix("PG_TEST").collect()?;
    config.set("validator.postgres", pg)?;
    Ok(NodeConfig::load_from(&config, false)?)
}

/// Create DB pool for automated tests
/// Pool is configured via PG_TEST_* env vars prefix
/// See [`deadpool_postgres::config::Config`] for full list of params
pub fn build_test_pool() -> anyhow::Result<Pool> {
    let config = build_test_config()?;
    let config: DeadpoolConfig = config.postgres;
    Ok(config.create_pool(NoTls)?)
}

/// Drops the db in the Config, creates it and runs the migrations
pub async fn reset_db(config: &NodeConfig, pool: &Pool) -> anyhow::Result<()> {
    let client = pool.get().await?;
    client.query("DROP SCHEMA public CASCADE;", &[]).await?;
    client.query("CREATE SCHEMA public;", &[]).await?;
    client.query("GRANT ALL ON SCHEMA public TO postgres;", &[]).await?;
    client.query("GRANT ALL ON SCHEMA public TO public;", &[]).await?;
    migrate(config.clone()).await?;

    Ok(())
}
