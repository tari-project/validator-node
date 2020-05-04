use structopt::StructOpt;
use tari_validator_node::{
    cli::{Arguments, Commands},
    config::NodeConfig,
    db::{migrations, utils},
    server::actix_main,
};

#[actix_rt::main]
async fn main() -> anyhow::Result<()> {
    let mut args = Arguments::from_args();

    // initialize configuration files if needed
    args.init_configs()?;
    let config = args.bootstrap.load_configuration()?;

    // deriving our app configs
    let node_config = NodeConfig::load_from(&config, true)?;

    match args.command {
        Commands::Start => actix_main(node_config).await?,
        Commands::Init => {
            println!("Initializing database {:?}", node_config.postgres.dbname);
            utils::create_database(node_config).await?;
        },
        Commands::Migrate => {
            println!("Running migrations on database {:?}", node_config.postgres.dbname);
            migrations::migrate(node_config).await?;
        },
        Commands::Access(cmd) => {
            println!("Access -> {:?}", cmd);
            cmd.run(node_config).await?;
        },
        Commands::Wipe { y } => {
            if !y && !prompt("Do you really want to wipe all data (Y/n)?") {
                return Ok(());
            }
            println!("Resetting database {:?}", node_config.postgres.dbname);
            utils::reset_database(node_config).await?;
        },
    };

    Ok(())
}

fn prompt(question: &str) -> bool {
    println!("{}", question);
    let mut input = "".to_string();
    std::io::stdin().read_line(&mut input).unwrap();
    let input = input.trim().to_lowercase();
    input == "y" || input.is_empty()
}
