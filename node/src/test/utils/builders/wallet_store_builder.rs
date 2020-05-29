use crate::{test::utils::Test, wallet::WalletStore};
use std::sync::Arc;
use tempdir::TempDir;
use tokio::sync::Mutex;

pub struct WalletStoreBuilder;

impl WalletStoreBuilder {
    pub fn build() -> anyhow::Result<Arc<Mutex<WalletStore>>> {
        let wallets = WalletStore::init(Test::<TempDir>::get_path_buf())?;
        Ok(Arc::new(Mutex::new(wallets)))
    }
}
