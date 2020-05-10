use crate::{config::NodeConfig, db::migrations::migrate};
use config::Source;
use deadpool_postgres::{Client, Pool};
use tokio::sync::{Mutex, MutexGuard};
use tokio_postgres::NoTls;

pub(crate) mod builders;

lazy_static::lazy_static! {
    static ref LOCK_DB_POOL: Mutex<Pool> = {
        let config = build_test_config().expect("LOCK_DB_POOL: failed to create test config");
        let pool = config.postgres.create_pool(NoTls).expect("LOCK_DB_POOL: failed to create DB pool");
        Mutex::new(pool)
    };
}

pub fn load_env() {
    let _ = dotenv::dotenv();
}
/// Create DB pool, reset DB, lock DB fo concurrent access, returns client and lock
pub async fn test_db_client<'a>() -> (Client, MutexGuard<'a, Pool>) {
    load_env();
    let db = test_pool().await;
    let config = build_test_config().unwrap();
    reset_db(&config, &db).await.unwrap();
    (db.get().await.unwrap(), db)
}

/// Generate a standard test config
pub fn build_test_config() -> anyhow::Result<NodeConfig> {
    let mut config = config::Config::new();
    let pg = config::Environment::with_prefix("PG_TEST").collect()?;
    config.set("validator.postgres", pg)?;
    config.set(
        "validator.wallets_keys_path",
        format!("{}/wallets", option_env!("OUT_DIR").unwrap_or("./.tari")),
    )?;
    Ok(NodeConfig::load_from(&config, false)?)
}

/// Return DB pool for automated tests.
/// Pool is wrapped in Mutex to avoid DB tests race conditions.
/// Test pool is configured via PG_TEST_* env vars prefix
/// See [`deadpool_postgres::config::Config`] for full list of params
pub async fn test_pool<'a>() -> MutexGuard<'a, Pool> {
    LOCK_DB_POOL.lock().await
}

/// Drops the db in the Config, creates it and runs the migrations
pub async fn reset_db(config: &NodeConfig, pool: &Pool) -> anyhow::Result<()> {
    let client = pool.get().await?;
    client.query("DROP SCHEMA IF EXISTS public CASCADE;", &[]).await?;
    client.query("CREATE SCHEMA IF NOT EXISTS public;", &[]).await?;
    client.query("GRANT ALL ON SCHEMA public TO postgres;", &[]).await?;
    client.query("GRANT ALL ON SCHEMA public TO public;", &[]).await?;
    migrate(config.clone()).await?;

    Ok(())
}
