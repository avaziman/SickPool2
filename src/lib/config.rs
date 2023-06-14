use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

use crate::{p2p::networking::protocol::ConfigP2P, stratum::config::StratumConfig};

#[derive(Deserialize, Serialize, Debug)]
pub struct ServerConfig {
    pub address: SocketAddr,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ProtocolServerConfig<ProtocolConfig> {
    #[serde(flatten)]
    pub server_config: ServerConfig,
    #[serde(flatten)]
    pub protocol_config: ProtocolConfig,
}

impl Default for ProtocolServerConfig<StratumConfig> {
    fn default() -> Self {
        Self {
            server_config: ServerConfig {
                address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1948),
            },
            protocol_config: StratumConfig::default(),
        }
    }
}

impl Default for ProtocolServerConfig<ConfigP2P> {
    fn default() -> Self {
        Self {
            server_config: ServerConfig {
                address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1967),
            },
            protocol_config: Default::default(),
        }
    }
}
