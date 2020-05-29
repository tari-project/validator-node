use crate::{
    config::NodeConfig,
    db::utils::db::db_client,
    db::model::*,
};
use structopt::StructOpt;
use tari_common::GlobalConfig;

#[derive(StructOpt, Debug)]
pub enum TemplateCommands {
    /// List assets
    List,
    /// List asset tokens
    Asset(AssetCommands),
}

impl TemplateCommands {
    pub async fn run(self, node_config: NodeConfig, global_config: GlobalConfig) -> anyhow::Result<()> {
        match self {
            Self::List => {
                println!("{}", wallet);
            },
            Self::List => {
                let wallets = store.load(&client).await?;
                for wallet in wallets.iter() {
                    println!("{}\n", wallet);
                }
            },
            Self::Info { pubkey } => {
                let wallet = store.get(pubkey, &client).await?;
                println!("{}", wallet);
            },
        };
        Ok(())
    }
}
