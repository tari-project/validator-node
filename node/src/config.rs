use crate::server::ActixConfig;
use config::{Config, Environment, Source};
use deadpool_postgres::config::Config as DeadpoolConfig;
use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};
use tari_common::{ConfigurationError, DefaultConfigLoader, NetworkConfigPath};

pub const DEFAULT_DBNAME: &'static str = "validator";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NodeConfig {
    /// will load from [validator.actix], overloaded with ACTIX_* env vars
    pub actix: ActixConfig,
    /// will load from [validator.postgres], overloaded with PG_* env vars
    /// see [deadpool_postgres::config::Config] on env + config vars details
    #[serde(serialize_with = "default_postgres_config")]
    pub postgres: DeadpoolConfig,
}

impl NetworkConfigPath for NodeConfig {
    fn main_key_prefix() -> &'static str {
        "validator"
    }
}

impl NodeConfig {
    pub fn load_from(config: &Config, env: bool) -> Result<Self, ConfigurationError> {
        let mut config = config.clone();
        if env {
            let actix = Environment::with_prefix("ACTIX").collect()?;
            let pg = Environment::with_prefix("PG").collect()?;
            config.set("validator.actix", actix)?;
            config.set("validator.postgres", pg)?;
        }
        <Self as DefaultConfigLoader>::load_from(&config)
    }
}

// Database default parameters
fn default_postgres_config<S: Serializer>(_: &DeadpoolConfig, s: S) -> Result<S::Ok, S::Error> {
    let mut db = s.serialize_map(None)?;
    db.serialize_entry("dbname", DEFAULT_DBNAME)?;
    db.end()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::server::*;
    use config::{Config, File, FileFormat::Toml};

    lazy_static::lazy_static! {
    static ref LOCK_ENV: std::sync::RwLock<u8> = std::sync::RwLock::new(0);
    }

    #[test]
    fn default_config() -> Result<(), ConfigurationError> {
        let _guard = LOCK_ENV.read().unwrap();
        let cfg = NodeConfig::load_from(&Config::new(), false)?;
        assert_eq!(cfg.actix.port, DEFAULT_PORT);
        assert_eq!(cfg.actix.host, DEFAULT_ADDR);
        assert_eq!(cfg.postgres.host, None);
        assert_eq!(cfg.postgres.dbname, Some(DEFAULT_DBNAME.into()));
        Ok(())
    }

    const TEST_CONFIG: &'static str = r#"
    [validator]
    actix = { workers = 3, port = 9999 }
    postgres = { host = "localhost", user = "postgres" }
    "#;

    #[test]
    fn load_config() -> Result<(), ConfigurationError> {
        let _guard = LOCK_ENV.read().unwrap();
        let mut settings = Config::new();
        settings.merge(File::from_str(TEST_CONFIG, Toml))?;

        let cfg = NodeConfig::load_from(&settings, false)?;
        assert_eq!(cfg.actix.port, 9999);
        assert_eq!(cfg.actix.host, DEFAULT_ADDR);
        assert_eq!(cfg.actix.workers, Some(3));
        assert_eq!(cfg.postgres.host, Some("localhost".into()));
        assert_eq!(cfg.postgres.dbname, Some(DEFAULT_DBNAME.into()));
        assert_eq!(cfg.postgres.user, Some("postgres".into()));
        assert_eq!(cfg.postgres.password, None);
        Ok(())
    }

    const TEST_CONFIG_NETWORK: &'static str = r#"
    use_network = "rincewind"
    [validator.rincewind]
    actix = { host = "10.0.0.1" }
    postgres = { host = "postgres", dbname = "validator_rincewind" }
    "#;

    #[test]
    fn network_overload_config() -> Result<(), ConfigurationError> {
        use std::net::IpAddr;

        let _guard = LOCK_ENV.read().unwrap();
        let mut settings = Config::new();
        let cfg_with_network = format!("{}{}", TEST_CONFIG, TEST_CONFIG_NETWORK);
        settings.merge(File::from_str(cfg_with_network.as_str(), Toml))?;

        let cfg = NodeConfig::load_from(&settings, false)?;
        assert_eq!(cfg.actix.port, 9999);
        assert_eq!(cfg.actix.host, "10.0.0.1".parse::<IpAddr>().unwrap());
        assert_eq!(cfg.actix.workers, Some(3));
        assert_eq!(cfg.postgres.host, Some("postgres".into()));
        assert_eq!(cfg.postgres.dbname, Some("validator_rincewind".into()));
        assert_eq!(cfg.postgres.user, Some("postgres".into()));
        assert_eq!(cfg.postgres.password, None);
        Ok(())
    }

    #[test]
    fn env_overload_config() -> Result<(), ConfigurationError> {
        // make sure that env settings do not interfere with other tests
        let _guard = LOCK_ENV.write().unwrap();
        let mut settings = Config::new();
        settings.merge(File::from_str(TEST_CONFIG, Toml))?;
        std::env::remove_var("PG_USER");
        std::env::remove_var("PG_DBNAME");
        std::env::set_var("PG_HOST", "postgres");
        std::env::set_var("PG_PASSWORD", "pass");
        std::env::set_var("ACTIX_WORKERS", "5");
        std::env::set_var("ACTIX_PORT", "5000");

        let cfg = NodeConfig::load_from(&settings, true)?;
        assert_eq!(cfg.actix.port, 5000);
        assert_eq!(cfg.actix.host, DEFAULT_ADDR);
        assert_eq!(cfg.actix.workers, Some(5));
        assert_eq!(cfg.postgres.host, Some("postgres".into()));
        assert_eq!(cfg.postgres.dbname, Some(DEFAULT_DBNAME.into()));
        assert_eq!(cfg.postgres.user, Some("postgres".into()));
        assert_eq!(cfg.postgres.password, Some("pass".into()));

        std::env::remove_var("PG_PASSWORD");
        std::env::remove_var("PG_HOST");
        std::env::remove_var("ACTIX_WORKERS");
        std::env::remove_var("ACTIX_PORT");

        Ok(())
    }
}