use std::net::{SocketAddr};

use serde::{Deserialize, Serialize};



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