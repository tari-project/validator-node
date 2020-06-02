use actix::Actor;
use dotenv::dotenv;
use structopt::StructOpt;
use tari_common::GlobalConfig;
use tari_validator_node::{
    api::server::actix_main,
    config::NodeConfig,
    db::{migrations, utils},
    metrics::Metrics,
};
use tvnc::{console::ServerConsole, Arguments, Commands};

async fn start_server(node_config: NodeConfig) -> anyhow::Result<()> {
    let metrics_addr = Metrics::default().start();
    let kill_console = ServerConsole::init(metrics_addr.clone()).await;
    let res = actix_main(node_config, Some(metrics_addr), kill_console).await;
    log::debug!("Terminating console: {:?}", res);
    res
}

#[actix_rt::main]
async fn main() -> anyhow::Result<()> {
    let mut args = Arguments::from_args();
    dotenv().ok();

    // initialize configuration files if needed
    args.init_configs()?;
    let config = args.load_configuration()?;
    let global_config = GlobalConfig::convert_from(config.clone())?;
    let node_config = NodeConfig::load_from(&config, &global_config, true)?;

    match args.command {
        Commands::Start => start_server(node_config).await?,
        Commands::Init => {
            println!("Initializing database {:?}", node_config.postgres.dbname);
            utils::db::create_database(node_config).await?;
        },
        Commands::Migrate => {
            println!("Running migrations on database {:?}", node_config.postgres.dbname);
            migrations::migrate(node_config).await?;
        },
        Commands::Access(cmd) => {
            println!("Access -> {:?}", cmd);
            cmd.run(node_config).await?;
        },
        Commands::Wallet(cmd) => {
            println!("Wallet -> {:?}", cmd);
            cmd.run(node_config, global_config).await?;
        },
        Commands::Wipe { y } => {
            if !y && !prompt("Do you really want to wipe all data (Y/n)?") {
                return Ok(());
            }
            println!("Resetting database {:?}", node_config.postgres.dbname);
            utils::db::reset_database(node_config).await?;
        },
        Commands::Template(cmd) => {
            println!("Template -> {:?}", cmd);
            cmd.run(node_config).await?;
        },
        Commands::Instruction(cmd) => {
            println!("Instruction -> {:?}", cmd);
            cmd.run(node_config).await?;
        },
        Commands::Asset(cmd) => {
            println!("Asset -> {:?}", cmd);
            cmd.run(node_config).await?;
        },
        Commands::Token(cmd) => {
            println!("Token -> {:?}", cmd);
            cmd.run(node_config).await?;
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
