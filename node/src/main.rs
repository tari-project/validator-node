use actix_rt::Runtime;
use structopt::StructOpt;
use tari_validator_node::{
    cli::{Arguments, Commands},
    config::NodeConfig,
    db::migrations,
    server::actix_main,
};

fn main() -> anyhow::Result<()> {
    let mut args = Arguments::from_args();

    // initialize configuration files if needed
    args.bootstrap.init_dirs()?;
    args.bootstrap.initialize_logging()?;
    let config = args.bootstrap.load_configuration()?;

    // deriving our app configs
    let node_config = NodeConfig::load_from(&config, true)?;

    let mut rt = Runtime::new().unwrap();
    match args.command {
        Commands::Start => actix_main(node_config)?,
        Commands::Migrate => {
            println!("Running migrations");
            rt.block_on(migrations::migrate(node_config))?;
        },
        Commands::Access(cmd) => {
            println!("Access -> {:?}", cmd);
            rt.block_on(cmd.run(node_config))?;
        },
    };

    Ok(())
}
