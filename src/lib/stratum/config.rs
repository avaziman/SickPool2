use std::{time::Duration, net::{SocketAddr, IpAddr, Ipv4Addr}};

use duration_str::deserialize_duration;
use serde::{Deserialize, Serialize};

use crate::{config::{ProtocolServerConfig, ServerConfig}, p2p::networking::hard_config::DEFAULT_STRATUM_PORT};

#[derive(Serialize, Deserialize, Debug)]
pub struct StratumConfig {
    pub rpc_url: String,
    #[serde(deserialize_with = "deserialize_duration")]
    pub job_poll_interval: Duration,
    pub default_diff_units: u64,
}

impl Default for StratumConfig {
    fn default() -> Self {
        Self {
            rpc_url: String::from(""),
            job_poll_interval: Duration::from_secs(5),
            default_diff_units: 1000,
        }
    }
}

impl Default for ProtocolServerConfig<StratumConfig> {
    fn default() -> Self {
        ProtocolServerConfig {
            server_config: ServerConfig {
                address: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), DEFAULT_STRATUM_PORT),
                processing_threads: 2,
            },
            protocol_config: StratumConfig::default(),
        }
    }
}
