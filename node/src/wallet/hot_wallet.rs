use crate::{db::models::wallet::*, errors::WalletError, types::Pubkey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use tari_comms::{multiaddr::Multiaddr, peer_manager::PeerFeatures, types::CommsPublicKey, NodeIdentity};
use tari_core::{
    tari_utilities::hex::Hex,
    transactions::{crypto::keys::SecretKey as SK, types::PrivateKey},
};
use tari_wallet::util::emoji::EmojiId;

/// Newly Generated tari wallet identity, used to initialize HotWallet
#[derive(Serialize, Deserialize, Clone)]
pub struct NodeWallet {
    identity: NodeIdentity,
    name: String,
}

impl std::fmt::Display for NodeWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Name: {}\n", self.name)?;
        write!(f, "{}", self.identity)?;
        let emoji_id = EmojiId::from_pubkey(self.identity.public_key());
        write!(f, "Emoji ID: {}", emoji_id)
    }
}

impl NodeWallet {
    /// Create a new encapsulated [`NodeIdentity`]
    /// ## Parameters
    /// `public_addr` - Network address of the base node
    pub fn new(public_addr: Multiaddr, name: String) -> Result<Self, WalletError> {
        let private_key = PrivateKey::random(&mut OsRng);
        let identity = NodeIdentity::new(private_key, public_addr, PeerFeatures::COMMUNICATION_CLIENT)?;
        Ok(Self { identity, name })
    }

    /// Generated public key hex
    #[inline]
    pub fn public_key_hex(&self) -> Pubkey {
        self.identity.public_key().to_hex()
    }
}

impl From<&NodeWallet> for NewWallet {
    fn from(source: &NodeWallet) -> Self {
        Self {
            pub_key: source.public_key_hex(),
            name: source.name.clone(),
        }
    }
}

/// Shared wallet entity, keeps track of wallet keys, attributes and balance
#[derive(Clone)]
pub struct HotWallet {
    id: NodeWallet,
    data: Wallet,
    /* Notes for later when linking to base node Wallet:
     * The easiest way to embed wallet is via Wallet struct - it produces a container that holds all
     * the handles you need to speak to the various services using their Service APIs
     * https://github.com/tari-project/tari/blob/development/base_layer/wallet/src/wallet.rs#L139
     * The wallet itself doesn't "sync" the blockchain. all the functions in the wallet that say Sync
     * tend to be sending a message to a Base Node to check if their outputs exist in the Base nodes chain db
     * We will need to run base node too though we can reuse that instance across all wallets - the
     * fn set_base_node_peer(...) in the Wallet struct where you tell the wallets about the Base Node
     * Multi-wallet tests can serve as example:
     *  https://github.com/tari-project/tari/blob/development/base_layer/wallet/tests/wallet/mod.rs */
}

impl std::fmt::Display for HotWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UUID: {}\n", self.data.id)?;
        write!(f, "{}", self.id)?;
        write!(f, "Balance: {}", self.data.balance)
    }
}

impl HotWallet {
    /// HotWallet's should be loaded via [`WalletStore`], created via [`NodeWallet`]
    pub(crate) fn new(id: NodeWallet, data: Wallet) -> Self {
        Self { id, data }
    }

    /// Wallet's node identity
    #[inline]
    pub fn identity(&self) -> &NodeWallet {
        &self.id
    }

    /// Wallet public key
    #[inline]
    pub fn public_key_hex(&self) -> Pubkey {
        self.id.public_key_hex()
    }

    /// Wallet public key
    #[inline]
    pub fn public_key(&self) -> &CommsPublicKey {
        self.id.identity.public_key()
    }

    /// Wallet name
    #[inline]
    pub fn name(&self) -> &String {
        &self.data.name
    }

    /// Wallet balance
    #[inline]
    pub fn balance(&self) -> i64 {
        self.data.balance
    }
}
