use crate::wallet::WalletStore;
use tari_test_utils::random::string;
use tempdir::TempDir;

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
    pub fn build(self) -> anyhow::Result<WalletStore> {
        Ok(WalletStore::init(self.temp_dir.path().to_path_buf())?)
    }
}
