use tari_common::{load_configuration, initialize_logging};
use std::fmt;
use super::cli::Arguments;
use config::Config;

#[derive(Debug)]
pub struct ConfigError {
    cause: &'static str,
    source: Option<String>,
}
impl std::error::Error for ConfigError {}
impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Configuration failed: {}", self.cause)?;
        if let Some(ref source) = self.source {
            write!(f, ": {}", source)
        } else {
            Ok(())
        }
    }
}

impl Arguments {
    pub fn initialize_logging(&self) -> Result<(), ConfigError> {
        match initialize_logging(&self.bootstrap.log_config) {
            false => Err(ConfigError { cause: "failed to initialize logging subsystem", source: None }),
            true => Ok(())
        }
    }

    pub fn load_configuration(&self) -> Result<Config, ConfigError> {
        match load_configuration(&self.bootstrap) {
            Err(err) => {
//                error!("", );
                Err(ConfigError { cause:  "failed to load configuration", source: Some(err)})
            },
            Ok(config) => Ok(config)
        }
    }
}