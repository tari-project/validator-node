use structopt::StructOpt;
use tari_common::ConfigBootstrap;

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
    /// Start node
    Start,
    /// Run the migrations
    Migrate,
    /// API access management
    Access(AccessCommands),
}
impl Default for Commands {
    fn default() -> Self {
        Commands::Start
    }
}
