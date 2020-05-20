use std::net::{IpAddr, Ipv4Addr, ToSocketAddrs};

pub const DEFAULT_PORT: u16 = 3001;
pub const DEFAULT_ADDR: Ipv4Addr = Ipv4Addr::LOCALHOST;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActixConfig {
    pub host: IpAddr,
    pub port: u16,
    pub workers: Option<usize>,
    pub backlog: Option<usize>,
    pub maxconn: Option<usize>,
}
impl Default for ActixConfig {
    fn default() -> Self {
        Self {
            host: DEFAULT_ADDR.into(),
            port: DEFAULT_PORT,
            workers: None,
            backlog: None,
            maxconn: None,
        }
    }
}
impl ActixConfig {
    pub fn addr(&self) -> impl ToSocketAddrs {
        (self.host, self.port)
    }
}
