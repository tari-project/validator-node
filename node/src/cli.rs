use structopt::StructOpt;
use tari_common::ConfigBootstrap;

#[derive(StructOpt, Default, Debug)]
/// The reference Tari cryptocurrency validation node implementation
pub struct Arguments {
    #[structopt(flatten)]
    pub bootstrap: ConfigBootstrap,
    /// Create and save new node identity if one doesn't exist
    #[structopt(long)]
    pub create_id: bool,
    /// Run the migrations
    #[structopt(long)]
    pub run_migrations: bool,
}
