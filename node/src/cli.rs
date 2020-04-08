use tari_common::{bootstrap_config_from_cli, ConfigBootstrap};
use structopt::StructOpt;
use std::path::PathBuf;
use std::fmt;

// this just mocks ConfigBootstrap, ideally to derive directly to 
// TODO: move impl to commons
#[derive(StructOpt, Default, Debug)]
struct ConfigBootstrapOpt {
    /// A path to a directory to store your files
    #[structopt(long)]
    pub base_path: Option<PathBuf>,
    /// A path to the configuration file to use (config.toml)
    #[structopt(long)]
    pub config: Option<PathBuf>,
    /// The path to the log configuration file. It is set using the following precedence set:
    ///   1. from the command-line parameter,
    ///   2. from the `TARI_LOG_CONFIGURATION` environment variable,
    ///   3. from a default value, usually `~/.tari/log4rs.yml` (or OS equivalent).
    #[structopt(long, env = "TARI_LOG_CONFIGURATION")]
    pub log_config: Option<PathBuf>,
}

#[derive(StructOpt)]
/// The reference Tari cryptocurrency validation node implementation
pub struct Arguments {
    #[structopt(flatten)]
    _bootstrap: ConfigBootstrapOpt,
    /// Create and save new node identity if one doesn't exist
    #[structopt(long)]
    pub create_id: bool,
    /// Create a default configuration file if it doesn't exist
    #[structopt(long)]
    pub init: bool,
    #[structopt(skip)]
    pub bootstrap: ConfigBootstrap,
}

impl Default for Arguments {
    fn default() -> Self {
        Self {
            _bootstrap: Default::default(),
            create_id: false,
            init: false,
            bootstrap: bootstrap_config_from_cli(&Self::clap().get_matches()),
        }
    }
}

impl fmt::Debug for Arguments {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Arguments")
         .field("create_id", &self.create_id)
         .field("init", &self.init)
         .field("base_path", &self.bootstrap.base_path)
         .field("config", &self.bootstrap.config)
         .field("log_config", &self.bootstrap.log_config)
         .field("_bootstrap", &self._bootstrap)
         .finish()
    }
}

impl Arguments {
    pub fn config_bootstrap() -> ConfigBootstrap {
        bootstrap_config_from_cli(&Self::clap().get_matches())
    }
}
