use config::Config;
use serde::Deserialize;
use super::Transport;

#[derive(Deserialize)]
struct Config {
    transport: Transport,
}

const CONFIG_DEFAULT: &'static str = "config/default";

impl Config {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();

        s.merge(File::with_name(CONFIG_DEFAULT))?;

        let env = env::var("env").unwrap_or("development".into());
        s.merge(File::with_name(&format!("config/{}", env)).required(false))?;

        s.merge(Environment::with_prefix("tari"))?;

        s.try_into()
    }
}
