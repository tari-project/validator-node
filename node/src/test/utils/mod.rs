use crate::{config::NodeConfig, db::migrations::migrate};
use config::Source;
use deadpool_postgres::{Client, Pool};
use std::sync::Arc;
use tari_common::{default_config, ConfigBootstrap, GlobalConfig};
use tari_test_utils::random::string;
use tokio::sync::{Mutex, MutexGuard};
use tokio_postgres::NoTls;

pub mod actix;
pub mod builders;
mod types;

lazy_static::lazy_static! {
    static ref LOCK_DB_POOL: Mutex<Pool> = {
        let config = build_test_config().expect("LOCK_DB_POOL: failed to create test config");
        let pool = config.postgres.create_pool(NoTls).expect("LOCK_DB_POOL: failed to create DB pool");
        Mutex::new(pool)
    };
    static ref ACTIX_DB_POOL: Arc<Pool> = {
        let config = build_test_config().expect("ACTIX_DB_POOL: failed to create test config");
        let pool = config.postgres.create_pool(NoTls).expect("ACTIX_DB_POOL: failed to create DB pool");
        Arc::new(pool)
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
    reset_db(&config, &db).await;
    (db.get().await.unwrap(), db)
}

/// Generate a standard test config
pub fn build_test_global_config() -> anyhow::Result<GlobalConfig> {
    let temp_dir = tempdir::TempDir::new(string(8).as_str())?;
    let bootstrap = ConfigBootstrap {
        base_path: temp_dir.into_path(),
        ..Default::default()
    };
    Ok(GlobalConfig::convert_from(default_config(&bootstrap))?)
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
    let global = build_test_global_config()?;
    let config = NodeConfig::load_from(&config, &global, false)?;
    log::trace!(target: "test_utils", "Load test config: {:?}", config);
    Ok(config)
}

/// Return DB pool for automated tests.
/// Pool is wrapped in Mutex to avoid DB tests race conditions.
/// Test pool is configured via PG_TEST_* env vars prefix
/// See [`deadpool_postgres::config::Config`] for full list of params
pub async fn test_pool<'a>() -> MutexGuard<'a, Pool> {
    LOCK_DB_POOL.lock().await
}

pub fn actix_test_pool() -> Arc<Pool> {
    ACTIX_DB_POOL.clone()
}

/// Drops the db in the Config, creates it and runs the migrations
pub async fn reset_db(config: &NodeConfig, pool: &Pool) {
    let client = pool.get().await.unwrap();
    client
        .query("DROP SCHEMA IF EXISTS public CASCADE;", &[])
        .await
        .unwrap();
    client.query("CREATE SCHEMA IF NOT EXISTS public;", &[]).await.unwrap();
    client
        .query("GRANT ALL ON SCHEMA public TO postgres;", &[])
        .await
        .unwrap();
    client
        .query("GRANT ALL ON SCHEMA public TO public;", &[])
        .await
        .unwrap();
    migrate(config.clone()).await.unwrap();
}
