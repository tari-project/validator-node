use structopt::StructOpt;
use tari_common::{ConfigBootstrap, ConfigError};

mod access;
pub use access::AccessCommands;

#[derive(StructOpt, Default, Debug)]
/// The reference Tari cryptocurrency validation node implementation
pub struct Arguments {
    #[structopt(flatten)]
    pub bootstrap: ConfigBootstrap,
    #[structopt(subcommand)]
    pub command: Commands,
}

#[derive(StructOpt, Debug)]
pub enum Commands {
    /// Init configs and create the database, also running migrations
    Init,
    /// Start node
    Start,
    /// Run the migrations
    Migrate,
    /// API access management
    Access(AccessCommands),
    /// Recreate and migrate database,  *DANGER!* it will wipe all data
    Wipe {
        /// Don't prompt for confirmation
        #[structopt(short)]
        y: bool
    },
}
impl Default for Commands {
    fn default() -> Self {
        Commands::Start
    }
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
        Ok(())
    }
}
