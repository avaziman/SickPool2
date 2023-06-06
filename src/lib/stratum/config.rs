use std::net::SocketAddr;

pub struct StratumConfig {
    pub stratum_address: SocketAddr,
    pub rpc_url: String,
}