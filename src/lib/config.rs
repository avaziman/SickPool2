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