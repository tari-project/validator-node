use crate::wallet::NodeWallet;
use multiaddr::Multiaddr;

pub struct NodeWalletBuilder {
    pub address: Multiaddr,
    pub name: String,
}

impl Default for NodeWalletBuilder {
    fn default() -> Self {
        Self {
            address: Multiaddr::empty(),
            name: "test wallet".into(),
        }
    }
}

impl NodeWalletBuilder {
    #[allow(dead_code)]
    pub fn build(self) -> anyhow::Result<NodeWallet> {
        Ok(NodeWallet::new(self.address, self.name)?)
    }
}
