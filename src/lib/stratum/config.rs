use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct StratumConfig {
    pub rpc_url: String,
    pub job_poll_interval_ms: u64,
    pub default_diff_units: u64,
}