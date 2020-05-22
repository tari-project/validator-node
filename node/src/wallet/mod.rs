//! Wallet operations

use crate::{db::models::wallet::*, errors::WalletError};
use deadpool_postgres::{Client, Transaction};
use log::info;
use std::{collections::HashMap, path::PathBuf};

mod hot_wallet;
pub use hot_wallet::{HotWallet, NodeWallet};

const LOG_TARGET: &'static str = "wallet";

// TODO: convert to interior mutability?
/// Handles wallet storage operations, keeping FS and DB in sync
/// [`WalletStore`] is the only way to access [`HotWallet`] object
pub struct WalletStore {
    wallets_keys_path: PathBuf,
    cache: HashMap<String, HotWallet>,
}

impl WalletStore {
    /// Initialize store
    pub fn init(wallets_keys_path: PathBuf) -> Result<Self, WalletError> {
        Ok(Self {
            wallets_keys_path,
            cache: HashMap::new(),
        })
    }

    /// Add wallet to the file store and database
    pub async fn add<'t>(&mut self, wallet: NodeWallet, trans: &Transaction<'t>) -> Result<HotWallet, WalletError> {
        let data = NewWallet::from(&wallet);
        let model = Wallet::insert(data, trans).await?;
        let wallet = HotWallet::new(wallet, model);
        let pubkey = wallet.public_key_hex();
        let path = self.wallet_path(&pubkey);
        let writer = std::fs::File::create(path)?;
        serde_json::to_writer(writer, wallet.identity())?;
        self.cache.insert(pubkey, wallet.clone());
        Ok(wallet)
    }

    /// Load and return wallet, will try to load wallet from disk if not found in cache.
    ///
    /// ## Parameters
    /// `pubkey` - Wallet's public key
    pub async fn get(&mut self, pubkey: String, client: &Client) -> Result<HotWallet, WalletError> {
        if let Some(wallet) = self.cache.get(&pubkey) {
            return Ok(wallet.clone());
        }

        let path = self.wallet_path(&pubkey);
        if !path.exists() {
            return Err(WalletError::not_found(pubkey));
        }
        let id_str = std::fs::read_to_string(path)?;
        let id: NodeWallet = serde_json::from_str(&id_str)?;
        let model = Wallet::select_by_key(&pubkey, client).await?;
        let wallet = HotWallet::new(id, model);
        info!(
            target: LOG_TARGET,
            "Wallet loaded with public key {}",
            wallet.public_key_hex()
        );

        self.cache.insert(pubkey, wallet.clone());
        Ok(wallet)
    }

    /// Load all registerd wallets from the DB
    pub async fn load(&mut self, client: &Client) -> Result<Vec<HotWallet>, WalletError> {
        let all = SelectWallet::default();
        let wallets = Wallet::select(all, client).await?;
        let mut res = Vec::with_capacity(wallets.len());
        for wallet in wallets.into_iter() {
            let id = self.load_id(&wallet.pub_key).await?;
            res.push(HotWallet::new(id, wallet));
        }
        Ok(res)
    }

    /// Load [`NodeWallet`] from disk
    async fn load_id(&mut self, pubkey: &String) -> Result<NodeWallet, WalletError> {
        if let Some(wallet) = self.cache.get(pubkey) {
            return Ok(wallet.identity().clone());
        }
        let path = self.wallet_path(pubkey);
        if !path.exists() {
            return Err(WalletError::not_found(pubkey.clone()));
        }
        let id_str = std::fs::read_to_string(path)?;
        let id = serde_json::from_str(&id_str)?;
        info!(target: LOG_TARGET, "NodeWallet loaded with public key {}", pubkey);
        Ok(id)
    }

    fn wallet_path(&self, pubkey: &String) -> PathBuf {
        let filename = format!("{}.json", pubkey);
        self.wallets_keys_path.join(filename)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::test_db_client;
    use multiaddr::Multiaddr;
    use tari_core::tari_utilities::hex::Hex;
    use tari_test_utils::random::string;
    use tempdir::TempDir;

    #[actix_rt::test]
    async fn general_usage() -> anyhow::Result<()> {
        let temp_dir = TempDir::new(string(8).as_str())?;
        let (mut client, _lock) = test_db_client().await;
        let address = Multiaddr::empty();

        let mut store = WalletStore::init(temp_dir.path().to_path_buf())?;
        let wallet = NodeWallet::new(address, "taris".into())?;
        let pubkey = wallet.public_key_hex();
        let transaction = client.transaction().await?;
        store.add(wallet.clone(), &transaction).await?;
        transaction.commit().await?;
        let count = store.load(&client).await?.len();
        assert_eq!(count, 1);

        let wallet = store.get(pubkey.clone(), &client).await?;
        assert_eq!(wallet.name(), "taris");
        assert_eq!(wallet.public_key().to_hex(), pubkey);

        temp_dir.close()?;
        Ok(())
    }

    #[actix_rt::test]
    async fn duplicate_key() -> anyhow::Result<()> {
        let temp_dir = TempDir::new(string(8).as_str())?;
        let (mut client, _lock) = test_db_client().await;
        let address = Multiaddr::empty();

        let mut store = WalletStore::init(temp_dir.path().to_path_buf())?;
        let wallet = NodeWallet::new(address, "taris".to_string())?;

        let transaction = client.transaction().await?;
        store.add(wallet.clone(), &transaction).await?;
        transaction.commit().await?;
        let transaction = client.transaction().await?;
        store.add(wallet, &transaction).await?;
        transaction.commit().await?;

        let count = store.load(&client).await?.len();
        assert_eq!(count, 1);

        temp_dir.close()?;
        Ok(())
    }
}
