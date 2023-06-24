use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use serde::{Deserialize, Serialize};

use crate::{
    p2p::networking::{
        hard_config::{DEFAULT_P2P_PORT, DEFAULT_STRATUM_PORT},
        config::ConfigP2P,
    },
    stratum::config::StratumConfig,
};

#[derive(Deserialize, Serialize, Debug)]
pub struct ServerConfig {
    pub address: SocketAddr,
    pub processing_threads: u8,
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
                address: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), DEFAULT_STRATUM_PORT),
                processing_threads: 2,
            },
            protocol_config: StratumConfig::default(),
        }
    }
}

impl Default for ProtocolServerConfig<ConfigP2P> {
    fn default() -> Self {
        Self {
            server_config: ServerConfig {
                address: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), DEFAULT_P2P_PORT),
                processing_threads: 2,
            },
            protocol_config: Default::default(),
        }
    }
}
