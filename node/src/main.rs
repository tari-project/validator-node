pub mod cli;
pub mod config;
pub mod db;
pub mod server;
use self::{cli::Arguments, config::NodeConfig, server::actix_main};
use actix_rt::Runtime;
use db::migrations;
use structopt::StructOpt;
use tari_common::DefaultConfigLoader;

fn main() -> anyhow::Result<()> {
    let mut args = Arguments::from_args();

    // initialize configuration files if needed
    args.bootstrap.init_dirs()?;
    args.bootstrap.initialize_logging()?;
    let config = args.bootstrap.load_configuration()?;

    // deriving our app configs
    let node_config = <NodeConfig as DefaultConfigLoader>::load_from(&config)?;

    // Run any migrations that are outstanding
    if args.run_migrations {
        println!("Running migrations");
        let mut rt = Runtime::new().unwrap();
        rt.block_on(migrations::migrate(node_config.clone()));
    }

    actix_main(node_config)?;
    Ok(())
}
