use crate::console::Terminal;
use serde_json::json;
use structopt::StructOpt;
use tari_common::GlobalConfig;
use tari_validator_node::{
    config::NodeConfig,
    db::utils::db::db_client,
    wallet::{NodeWallet, WalletStore},
};

#[derive(StructOpt, Debug)]
pub enum WalletCommands {
    /// Create new wallet
    Create {
        /// Internal unique name of the wallet
        name: String,
    },
    /// List wallets available on this node
    List,
    /// Wallet details: key, balance, emoji
    View {
        /// Public key of a wallet
        pubkey: String,
    },
    /// Set wallet's balance to amount of micro-tari
    Balance {
        /// Public key of a wallet
        pubkey: String,
        /// New balance
        balance: i64,
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
                Terminal::basic().render_object("Wallet details", wallet.data().clone());
            },
            Self::List => {
                let wallets = store.load(&client).await?;
                let output: Vec<_> = wallets
                    .iter()
                    .map(|w| json!({"Pubkey": w.public_key(), "Name": w.name(), "Balance": w.balance()}))
                    .collect();
                Terminal::basic().render_list("Wallets", output, &["Pubkey", "Name", "Balance"], &[20, 40, 16]);
            },
            Self::View { pubkey } => {
                let wallet = store.get(pubkey, &client).await?;
                Terminal::basic().render_object("Wallet details", wallet.data().clone());
            },
            Self::Balance { pubkey, balance } => {
                let wallet = store.get(pubkey, &client).await?;
                let wallet = wallet.data().set_balance(balance, &client).await?;
                Terminal::basic().render_object("Wallet details", wallet);
            },
        };
        Ok(())
    }
}
