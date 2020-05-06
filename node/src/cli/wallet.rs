use crate::{
    config::NodeConfig,
    db::utils::db::db_client,
    wallet::{WalletID, WalletStore},
};
use structopt::StructOpt;
use tari_common::GlobalConfig;

#[derive(StructOpt, Debug)]
pub enum WalletCommands {
    /// Create new wallet
    Create {
        /// Internal unique name of the wallet
        #[structopt(short = "n", long)]
        name: String,
    },
    /// List wallets available on this node
    List,
    /// Wallet details: key, balance, emoji
    Info {
        /// Public key of a wallet
        #[structopt(short = "k", long)]
        pubkey: String,
    },
}

impl WalletCommands {
    pub async fn run(self, node_config: NodeConfig, global_config: GlobalConfig) -> anyhow::Result<()> {
        let mut client = db_client(&node_config).await?;
        let mut store = WalletStore::init(node_config.wallets_keys_path.clone())?;

        match self {
            Self::Create { name } => {
                let transaction = client.transaction().await?;
                let wallet = WalletID::new(global_config.public_address.clone(), name)?;
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
