use tari_common::ConfigBootstrap;
use structopt::StructOpt;

#[derive(StructOpt, Default, Debug)]
/// The reference Tari cryptocurrency validation node implementation
pub struct Arguments {
    #[structopt(flatten)]
    pub bootstrap: ConfigBootstrap,
    /// Create and save new node identity if one doesn't exist
    #[structopt(long)]
    pub create_id: bool,
}
