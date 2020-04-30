use super::errors::DBError;
use deadpool_postgres::{config::Config as DeadpoolConfig, Pool};
use tokio_postgres::NoTls;

pub fn build_pool(config: &DeadpoolConfig) -> Result<Pool, DBError> {
    Ok(config.create_pool(NoTls)?)
}

/// Create DB pool for automated tests
/// Pool is configured via PG_TEST_* env vars prefix
/// See [`deadpool_postgres::config::Config`] for full list of params
pub fn build_test_pool() -> anyhow::Result<Pool> {
    let mut config = config::Config::new();
    config.merge(config::Environment::with_prefix("PG_TEST"))?;
    let config: DeadpoolConfig = config.try_into()?;
    Ok(config.create_pool(NoTls)?)
}
