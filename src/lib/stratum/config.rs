use std::{time::Duration};

use duration_str::deserialize_duration;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct StratumConfig {
    pub rpc_url: String,
    #[serde(deserialize_with = "deserialize_duration")]
    pub job_poll_interval: Duration,
}

impl Default for StratumConfig {
    fn default() -> Self {
        Self {
            rpc_url: String::from(""),
            job_poll_interval: Duration::from_secs(5),
        }
    }
}