use crate::wallet::WalletStore;
use std::sync::Arc;
use tari_test_utils::random::string;
use tempdir::TempDir;
use tokio::sync::Mutex;

pub struct WalletStoreBuilder {
    pub temp_dir: TempDir,
}

impl Default for WalletStoreBuilder {
    fn default() -> Self {
        Self {
            temp_dir: TempDir::new(string(8).as_str()).unwrap(),
        }
    }
}

impl WalletStoreBuilder {
    pub fn build(self) -> anyhow::Result<Arc<Mutex<WalletStore>>> {
        let wallets = WalletStore::init(self.temp_dir.path().to_path_buf())?;
        Ok(Arc::new(Mutex::new(wallets)))
    }
}
