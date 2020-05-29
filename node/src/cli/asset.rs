use crate::{
    config::NodeConfig,
    db::utils::db::db_client,
    db::model::*,
};
use structopt::StructOpt;
use tari_common::GlobalConfig;

#[derive(StructOpt, Debug)]
pub enum AssetCommands {
    /// Create new asset
    Create {
        /// Name of the asset
        #[structopt(short = "n", long)]
        name: String,
        /// Descriptions
        #[structopt(short = "d", long)]
        description: String,
    },
    /// List assets
    List,
    /// List asset tokens
    Tokens {
        /// Public key of a wallet
        #[structopt(short = "k", long)]
        asset_id: AssetID,
    },
}

impl WalletCommands {
    pub async fn run(self, node_config: NodeConfig, global_config: GlobalConfig) -> anyhow::Result<()> {
        let mut client = db_client(&node_config).await?;
        let mut store = WalletStore::init(node_config.wallets_keys_path.clone())?;

        match self {
            Self::Create { name } => {
                let transaction = client.transaction().await?;
                let wallet = NodeWallet::new(global_config.public_address.clone(), name)?;
                let wallet = store.add(wallet, &transaction).await?;
                transaction.commit().await?;
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
