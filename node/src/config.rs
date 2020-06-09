use crate::{
    api::config::{ActixConfig, CorsConfig},
    consensus::ConsensusConfig,
    template::config::TemplateConfig,
};
use config::{Config, Environment, Source, Value};
use deadpool::managed::PoolConfig;
use deadpool_postgres::config::Config as DeadpoolConfig;
use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};
use tari_common::{ConfigurationError, DefaultConfigLoader, GlobalConfig, NetworkConfigPath};

pub const DEFAULT_DBNAME: &'static str = "validator";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NodeConfig {
    /// will load from [validator.actix], overloaded with ACTIX_* env vars
    pub actix: ActixConfig,
    /// will load from [validator.postgres], overloaded with PG_* env vars
    /// see [deadpool_postgres::config::Config] on env + config vars details
    #[serde(serialize_with = "default_postgres_config")]
    pub postgres: DeadpoolConfig,
    /// will load from [validator.cors], overloaded with CORS_* env vars
    pub cors: CorsConfig,
    /// Path to directory for storing wallets keys. Defaults to `~/.tari/wallets`
    pub wallets_keys_path: std::path::PathBuf,
    /// Node's public address. Defaults to [tari.public_address]
    pub public_address: Option<multiaddr::Multiaddr>,
    /// will load from [validator.consensus], overloaded with CONSENSUS_* env vars
    pub consensus: ConsensusConfig,
    /// will load from [validator.consensus], overloaded with CONSENSUS_* env vars
    pub template: TemplateConfig,
}

impl NetworkConfigPath for NodeConfig {
    fn main_key_prefix() -> &'static str {
        "validator"
    }
}

impl NodeConfig {
    pub fn load_from(config: &Config, global: &GlobalConfig, env: bool) -> Result<Self, ConfigurationError> {
        let mut config = config.clone();
        if env {
            let actix = Environment::with_prefix("ACTIX").collect()?;
            let pg = Environment::with_prefix("PG").collect()?;
            let cors = Environment::with_prefix("CORS").collect()?;
            let consensus = Environment::with_prefix("CONSENSUS").collect()?;
            let template = Environment::with_prefix("TEMPLATE").collect()?;
            config.set("validator.actix", actix).unwrap();
            config.set("validator.postgres", pg).unwrap();
            config.set("validator.cors", cors).unwrap();
            config.set("validator.consensus", consensus).unwrap();
            config.set("validator.template", template).unwrap();
            if let Some(pg_pool) = Self::pg_pool_from_env()? {
                config.set("validator.postgres.pool", pg_pool.collect()?).unwrap();
            }
        }
        Self::set_default(
            &mut config,
            "validator.public_address",
            global.public_address.to_string(),
        );
        Self::set_default(&mut config, "validator.postgres.manager.recycling_method", "fast");
        Self::set_default(
            &mut config,
            "validator.postgres.pool.max_size",
            PoolConfig::default().max_size as i64,
        );
        <Self as DefaultConfigLoader>::load_from(&config)
    }

    fn set_default<T: Into<Value>>(config: &mut Config, key: &str, value: T) {
        if config.get_str(key).is_err() {
            config.set(key, value).unwrap();
        }
    }

    // Workaround of buggy deadpool_postgres config env loader
    // TODO: this ideally should be fixed in deadpool config loader crate:
    fn pg_pool_from_env() -> Result<Option<Config>, ConfigurationError> {
        let pg_pool = Environment::with_prefix("PG_POOL").collect()?;
        if pg_pool.len() == 0 {
            return Ok(None);
        }
        let mut config = Config::new();
        let pg_pool_timeouts_recycle = Environment::with_prefix("PG_POOL_TIMEOUTS_RECYCLE").collect()?;
        let pg_pool_timeouts_create = Environment::with_prefix("PG_POOL_TIMEOUTS_CREATE").collect()?;
        let pg_pool_timeouts_wait = Environment::with_prefix("PG_POOL_TIMEOUTS_WAIT").collect()?;
        if pg_pool.len() > 0 && pg_pool.contains_key("max_size") {
            let max_size = pg_pool.get("max_size").unwrap().clone().into_int()?;
            config.set("max_size", max_size).unwrap();
        }
        if pg_pool_timeouts_wait.len() > 0 {
            if pg_pool_timeouts_wait.contains_key("secs") && pg_pool_timeouts_wait.contains_key("nanos") {
                config.set("timeouts.wait", pg_pool_timeouts_wait).unwrap();
            } else {
                panic!("PG_POOL_TIMEOUTS_WAIT should define _SECS and _NANOS suffixes")
            }
        }
        if pg_pool_timeouts_create.len() > 0 {
            if pg_pool_timeouts_create.contains_key("secs") && pg_pool_timeouts_create.contains_key("nanos") {
                config.set("timeouts.create", pg_pool_timeouts_create).unwrap();
            } else {
                panic!("PG_POOL_TIMEOUTS_CREATE should define _SECS and _NANOS suffixes")
            }
        }
        if pg_pool_timeouts_recycle.len() > 0 {
            if pg_pool_timeouts_recycle.contains_key("secs") && pg_pool_timeouts_recycle.contains_key("nanos") {
                config.set("timeouts.recycle", pg_pool_timeouts_recycle).unwrap();
            } else {
                panic!("PG_POOL_TIMEOUTS_RECYCLE should define _SECS and _NANOS suffixes")
            }
        }
        Ok(Some(config))
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
    use crate::{
        api::config::actix::{DEFAULT_ADDR, DEFAULT_PORT},
        test::utils::build_test_global_config,
    };
    use config::{Config, File, FileFormat::Toml};
    use deadpool_postgres::config::*;
    use std::time::Duration;

    lazy_static::lazy_static! {
    static ref LOCK_ENV: std::sync::RwLock<u8> = std::sync::RwLock::new(0);
    }

    #[test]
    fn default_config() {
        let global = build_test_global_config().unwrap();
        let cfg = NodeConfig::load_from(&Config::new(), &global, false).unwrap();
        assert_eq!(cfg.actix.port, DEFAULT_PORT);
        assert_eq!(cfg.actix.host, DEFAULT_ADDR);
        assert_eq!(cfg.postgres.host, None);
        assert_eq!(cfg.postgres.dbname, Some(DEFAULT_DBNAME.into()));
        assert_eq!(cfg.cors.allowed_origins, "*");
        assert_eq!(
            cfg.postgres.manager.map(|m| m.recycling_method),
            Some(RecyclingMethod::Fast)
        );
    }

    const TEST_CONFIG: &'static str = r#"
    [validator.postgres]
    host = "localhost"
    user = "postgres"
    pool = { timeouts = { wait = {secs = 5, nanos = 0} } }
    [validator]
    actix = { workers = 3, port = 9999 }
    cors = { allowed_origins = "https://www.tari.com"}
    consensus = { workers = 10 }
    template = { runner_max_jobs = 10 }
    "#;

    #[test]
    fn load_config() {
        let global = build_test_global_config().unwrap();
        let mut settings = Config::new();
        settings.merge(File::from_str(TEST_CONFIG, Toml)).unwrap();

        let cfg = NodeConfig::load_from(&settings, &global, false).unwrap();
        assert_eq!(cfg.actix.port, 9999);
        assert_eq!(cfg.actix.host, DEFAULT_ADDR);
        assert_eq!(cfg.actix.workers, Some(3));
        assert_eq!(cfg.postgres.host, Some("localhost".into()));
        assert_eq!(cfg.postgres.dbname, Some(DEFAULT_DBNAME.into()));
        assert_eq!(cfg.postgres.user, Some("postgres".into()));
        assert_eq!(cfg.postgres.password, None);
        assert_eq!(
            cfg.postgres.pool.map(|p| p.timeouts.wait).flatten(),
            Some(Duration::from_secs(5))
        );
        assert_eq!(cfg.cors.allowed_origins, "https://www.tari.com".to_string());
        assert_eq!(cfg.consensus.workers, Some(10));
        assert_eq!(cfg.template.runner_max_jobs, 10);
    }

    const TEST_CONFIG_NETWORK: &'static str = r#"
    use_network = "rincewind"
    [validator.rincewind]
    actix = { host = "10.0.0.1" }
    [validator.rincewind.postgres]
    host = "postgres"
    dbname = "validator_rincewind"
    pool = { max_size = 5 }
    "#;

    #[test]
    fn network_overload_config() {
        use std::net::IpAddr;

        let global = build_test_global_config().unwrap();
        let mut settings = Config::new();
        let cfg_with_network = format!("{}{}", TEST_CONFIG, TEST_CONFIG_NETWORK);
        settings.merge(File::from_str(cfg_with_network.as_str(), Toml)).unwrap();

        let cfg = NodeConfig::load_from(&settings, &global, false).unwrap();
        assert_eq!(cfg.actix.port, 9999);
        assert_eq!(cfg.actix.host, "10.0.0.1".parse::<IpAddr>().unwrap());
        assert_eq!(cfg.actix.workers, Some(3));
        assert_eq!(cfg.postgres.host, Some("postgres".into()));
        assert_eq!(cfg.postgres.dbname, Some("validator_rincewind".into()));
        assert_eq!(cfg.postgres.user, Some("postgres".into()));
        assert_eq!(cfg.postgres.password, None);
        assert_eq!(cfg.postgres.pool.clone().map(|p| p.max_size), Some(5));
        assert_eq!(
            cfg.postgres.pool.map(|p| p.timeouts.wait).flatten(),
            Some(Duration::from_secs(5))
        );
    }

    #[test]
    fn env_overload_config() {
        // make sure that env settings do not interfere with other tests
        let _guard = LOCK_ENV.write().unwrap();
        let global = build_test_global_config().unwrap();
        let mut settings = Config::new();
        settings.merge(File::from_str(TEST_CONFIG, Toml)).unwrap();
        std::env::remove_var("PG_USER");
        std::env::remove_var("PG_DBNAME");
        std::env::set_var("PG_HOST", "postgres");
        std::env::set_var("PG_PASSWORD", "pass");
        std::env::set_var("ACTIX_WORKERS", "5");
        std::env::set_var("ACTIX_PORT", "5000");

        let cfg = NodeConfig::load_from(&settings, &global, true).unwrap();
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
    }

    #[test]
    fn pool_env_overload_config() {
        // make sure that env settings do not interfere with other tests
        let _guard = LOCK_ENV.write().unwrap();
        let global = build_test_global_config().unwrap();
        let mut settings = Config::new();
        settings.merge(File::from_str(TEST_CONFIG, Toml)).unwrap();
        std::env::remove_var("PG_POOL_MAX_SIZE");
        std::env::remove_var("PG_POOL_TIMEOUTS_WAIT_SECS");
        std::env::remove_var("PG_POOL_TIMEOUTS_WAIT_NANOS");
        std::env::remove_var("PG_POOL_TIMEOUTS_RECYCLE_SECS");
        std::env::remove_var("PG_POOL_TIMEOUTS_RECYCLE_NANOS");
        std::env::remove_var("PG_POOL_TIMEOUTS_CREATE_SECS");
        std::env::remove_var("PG_POOL_TIMEOUTS_CREATE_NANOS");

        std::env::set_var("PG_POOL_MAX_SIZE", "3");
        let cfg = NodeConfig::load_from(&settings, &global, true).unwrap();
        let pool = cfg.postgres.pool.unwrap();
        assert_eq!(pool.max_size, 3);

        std::env::set_var("PG_POOL_TIMEOUTS_WAIT_SECS", "1");
        std::env::set_var("PG_POOL_TIMEOUTS_WAIT_NANOS", "0");
        let cfg = NodeConfig::load_from(&settings, &global, true).unwrap();
        let pool = cfg.postgres.pool.unwrap();
        assert_eq!(pool.timeouts.wait, Some(Duration::from_secs(1)));
        assert_eq!(pool.timeouts.recycle, None);
        assert_eq!(pool.timeouts.create, None);

        std::env::set_var("PG_POOL_TIMEOUTS_RECYCLE_SECS", "2");
        std::env::set_var("PG_POOL_TIMEOUTS_RECYCLE_NANOS", "0");
        let cfg = NodeConfig::load_from(&settings, &global, true).unwrap();
        let pool = cfg.postgres.pool.unwrap();
        assert_eq!(pool.timeouts.recycle, Some(Duration::from_secs(2)));

        std::env::set_var("PG_POOL_TIMEOUTS_CREATE_SECS", "3");
        std::env::set_var("PG_POOL_TIMEOUTS_CREATE_NANOS", "0");
        let cfg = NodeConfig::load_from(&settings, &global, true).unwrap();
        let pool = cfg.postgres.pool.unwrap();
        assert_eq!(pool.timeouts.create, Some(Duration::from_secs(3)));

        std::env::remove_var("PG_POOL_MAX_SIZE");
        std::env::remove_var("PG_POOL_TIMEOUTS_WAIT_SECS");
        std::env::remove_var("PG_POOL_TIMEOUTS_WAIT_NANOS");
        std::env::remove_var("PG_POOL_TIMEOUTS_RECYCLE_SECS");
        std::env::remove_var("PG_POOL_TIMEOUTS_RECYCLE_NANOS");
        std::env::remove_var("PG_POOL_TIMEOUTS_CREATE_SECS");
        std::env::remove_var("PG_POOL_TIMEOUTS_CREATE_NANOS");
    }
}
