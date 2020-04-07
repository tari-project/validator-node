use config::Config;
use serde::Deserialize;
use multiaddr::{Multiaddr, Protocol};
use tari_common::TorControlAuthentication;

#[derive(Deserialize)]
enum Transport {
    Tor({
        control_address: Multiaddr,
        control_auth: TorControlAuthentication,
        onion_port: u16,
        forward_address: Multiaddr,
        socks_address_override: Option<Multiaddr>,
    }),
}

impl Transport {
    fn connect(&self) {
        match self {
            Tor(conn) => {},
        };
    }
}