use tari_validator_node::{
    cli::Arguments,
    config::NodeConfig,
    server::actix_main,
};
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

    actix_main(node_config.clone())?;

    Ok(())
}

