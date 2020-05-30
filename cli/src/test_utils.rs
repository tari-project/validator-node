// This is temporary, until we move out node::test::utils to separate crate
use config::Source;
use tari_common::{default_config, dir_utils::default_path, ConfigBootstrap, GlobalConfig};
use tari_validator_node::config::NodeConfig;

#[derive(Clone)]
pub struct Test<T>(std::marker::PhantomData<T>);

impl Test<ConfigBootstrap> {
    fn get() -> ConfigBootstrap {
        ConfigBootstrap {
            base_path: Test::<TempDir>::get_path_buf(),
            ..Default::default()
        }
    }
}

use tari_test_utils::random::string;
use tempdir::TempDir;

lazy_static::lazy_static! {
    static ref TEMP_DIR: TempDir = TempDir::new(string(8).as_str()).unwrap();
}

impl Test<TempDir> {
    pub fn get_path_buf() -> std::path::PathBuf {
        TEMP_DIR.path().to_path_buf()
    }
}

/// Generate a standard test config
pub fn build_test_global_config() -> anyhow::Result<GlobalConfig> {
    let bootstrap = Test::<ConfigBootstrap>::get();
    Ok(GlobalConfig::convert_from(default_config(&bootstrap))?)
}

/// Generate a standard test config
pub fn build_test_config() -> anyhow::Result<NodeConfig> {
    let _ = dotenv::dotenv();
    let bootstrap = Test::<ConfigBootstrap>::get();
    let mut config = default_config(&bootstrap);
    let pg = config::Environment::with_prefix("PG_TEST").collect()?;
    let global = build_test_global_config()?;
    config.set("validator.postgres", pg)?;
    config.set(
        "validator.wallets_keys_path",
        default_path("wallets", Some(&bootstrap.base_path)).to_str(),
    )?;
    let config = NodeConfig::load_from(&config, &global, false)?;
    Ok(config)
}
