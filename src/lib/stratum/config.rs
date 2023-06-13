use std::{net::SocketAddr, time::Duration};

use duration_str::deserialize_duration;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct StratumConfig {
    pub stratum_address: SocketAddr,
    pub rpc_url: String,
    #[serde(deserialize_with = "deserialize_duration")]
    pub job_poll_interval: Duration,
}