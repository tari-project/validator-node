use serde::Deserialize;
use multiaddr::Multiaddr;
use tari_common::TorControlAuthentication;

#[derive(Deserialize, Debug)]
#[serde(tag = "transport", rename_all = "lowercase")]
pub enum Transport {
    Tor {
        tor_control_address: Multiaddr,
        #[serde(with = "serde_with::rust::display_fromstr")]
        tor_control_auth: TorControlAuthentication,
        tor_onion_port: u16,
        tor_forward_address: Multiaddr,
        tor_socks_address_override: Option<Multiaddr>,
    },
}
