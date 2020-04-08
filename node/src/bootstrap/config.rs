use config::{Config, ConfigError, File, Environment};
use serde::Deserialize;
use super::Transport;

#[derive(Deserialize, Debug)]
pub struct NodeConfig {
    transport: Transport,
}

const CONFIG_DEFAULT: &'static str = "config/default";

impl NodeConfig {
    pub fn init() -> Result<Self, ConfigError> {
        let mut s = Config::new();

        s.merge(File::with_name(CONFIG_DEFAULT))?;

        let env = std::env::var("env").unwrap_or("development".into());
        s.merge(File::with_name(&format!("config/{}", env)).required(false))?;

        s.merge(Environment::with_prefix("tari"))?;

        s.try_into()
    }
}
