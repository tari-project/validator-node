use super::{Commands, ConfigError};
use structopt::StructOpt;
use tari_common::{
    dir_utils::{create_data_directory, default_path},
    ConfigBootstrap,
};

#[derive(StructOpt, Default, Debug)]
/// The reference Tari cryptocurrency validation node implementation
pub struct Arguments {
    #[structopt(flatten)]
    pub bootstrap: ConfigBootstrap,
    /// Path to directory for storing wallets keys.
    /// Can be overloaded via env `VALIDATION_NODE_WALLETS`.
    /// Defaults to `~/.tari/wallets`.
    #[structopt(short, long, env = "VALIDATION_NODE_WALLETS")]
    pub wallets_keys_path: Option<std::path::PathBuf>,
    #[structopt(subcommand)]
    pub command: Commands,
}

impl Arguments {
    /// Initialize tari configuration and logger according to CLI params
    /// `node init` command will create configs without prompt (same as flag `--init`)
    pub fn init_configs(&mut self) -> Result<(), ConfigError> {
        match self.command {
            Commands::Init => self.bootstrap.init = true,
            _ => {},
        };
        self.bootstrap.init_dirs()?;
        self.bootstrap.initialize_logging()?;
        let wallet_path = self
            .wallets_keys_path
            .get_or_insert(default_path("wallets", Some(&self.bootstrap.base_path)));
        create_data_directory(Some(wallet_path))?;
        Ok(())
    }

    pub fn load_configuration(&mut self) -> Result<config::Config, ConfigError> {
        let mut config = self.bootstrap.load_configuration()?;
        if config.get_str("validator.wallets_keys_path").is_err() {
            let wallet_path = self
                .wallets_keys_path
                .get_or_insert(default_path("wallets", Some(&self.bootstrap.base_path)));
            config.set("validator.wallets_keys_path", wallet_path.to_str())?;
        };
        Ok(config)
    }
}
