use super::errors::DBError;
use crate::{config::NodeConfig, db::migrations::migrate};
use deadpool_postgres::{config::Config as DeadpoolConfig, Pool};
use tokio_postgres::{Config as PgConfig, NoTls};

pub fn build_pool(config: &DeadpoolConfig) -> Result<Pool, DBError> {
    Ok(config.create_pool(NoTls)?)
}

/// Creates to postgres database without the pool
pub async fn connect_raw(pg: PgConfig) -> Result<tokio_postgres::Client, DBError> {
    let (client, connection) = pg.connect(NoTls).await?;

    actix_rt::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    Ok(client)
}

/// Pick single DB client from a pool
pub async fn db_client(config: &NodeConfig) -> Result<deadpool_postgres::Client, DBError> {
    let pool = build_pool(&config.postgres)?;
    Ok(pool.get().await?)
}

/// Creates database for validator node.
/// Dataase name specified either as `PG_DBNAME` env
/// or `validator.postgres.dbname` config parameter
/// Defaults to `validator`
pub async fn create_database(config: NodeConfig) -> Result<(), DBError> {
    let mut pg = config.postgres.get_pg_config()?;
    let dbname = pg.get_dbname().unwrap_or("validator").to_string();

    pg.dbname("postgres");
    let client = connect_raw(pg).await?;

    let db_exists = client
        .query_one(
            "SELECT EXISTS(SELECT datname FROM pg_catalog.pg_database WHERE datname = $1)",
            &[&dbname],
        )
        .await?
        .get::<_, bool>(0);

    if db_exists == false {
        client
            .execute(format!("CREATE DATABASE {}", dbname).as_str(), &[])
            .await?;
    }

    migrate(config).await?;
    Ok(())
}

/// Resets database for validator node, it will wipe all data.
pub async fn reset_database(config: NodeConfig) -> Result<(), DBError> {
    let pg = config.postgres.get_pg_config()?;
    let client = connect_raw(pg).await?;

    client.query("DROP SCHEMA public CASCADE;", &[]).await?;
    client.query("CREATE SCHEMA public;", &[]).await?;
    client.query("GRANT ALL ON SCHEMA public TO postgres;", &[]).await?;
    client.query("GRANT ALL ON SCHEMA public TO public;", &[]).await?;

    migrate(config).await?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::reset_database;
    use crate::test::utils::{build_test_config, load_env, test_pool};

    #[actix_rt::test]
    async fn test_reset_database() -> anyhow::Result<()> {
        load_env();
        let _lock_db = test_pool().await;
        let config = build_test_config().unwrap();
        reset_database(config).await?;
        Ok(())
    }
}
