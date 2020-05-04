use super::errors::DBError;
use crate::{config::NodeConfig, db::migrations::migrate};
use tokio_postgres::{Client, Config as PgConfig, NoTls};

/// Creates to postgres database without the pool
pub async fn connect_raw(pg: PgConfig) -> Result<Client, DBError> {
    let (client, connection) = pg.connect(NoTls).await?;

    actix_rt::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    Ok(client)
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

    client
        .execute(format!("CREATE DATABASE {}", dbname).as_str(), &[])
        .await?;
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
    use crate::test_utils::build_test_config;

    #[actix_rt::test]
    async fn test_reset_database() -> anyhow::Result<()> {
        dotenv::dotenv().unwrap();
        let config = build_test_config().unwrap();
        reset_database(config).await?;
        Ok(())
    }
}
