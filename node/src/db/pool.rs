use super::utils::errors::DBError;
use deadpool_postgres::{config::Config as DeadpoolConfig, Pool};
use tokio_postgres::NoTls;

pub fn build_pool(config: &DeadpoolConfig) -> Result<Pool, DBError> {
    Ok(config.create_pool(NoTls)?)
}
