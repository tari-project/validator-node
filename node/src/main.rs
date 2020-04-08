use tari_validator_node::cli::Arguments;
use structopt::StructOpt;

fn main() -> anyhow::Result<()> {
    let args = Arguments::from_args();

    // Initialise the logger
    args.initialize_logging()?;

    // Load and apply configuration file
    args.load_configuration()?;

    dbg!(args);
    
    Ok(())
}
