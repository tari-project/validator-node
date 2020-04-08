use tari_validator_node::bootstrap::Config;

fn main() -> anyhow::Result<()> {
    let config = Config::init()?;
    dbg!(config);
    Ok(())
}
