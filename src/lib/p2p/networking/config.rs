use std::time::Duration;

use duration_str::deserialize_duration;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigP2P {
    pub peer_connections: u32,
    #[serde(flatten)]
    pub consensus: ConsensusConfigP2P,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConsensusConfigP2P {
    pub genesis_height: u32,
    #[serde(deserialize_with = "deserialize_duration")]
    pub block_time: Duration,
    pub diff_adjust_blocks: u32,
}

impl Default for ConsensusConfigP2P {
    fn default() -> Self {
        Self {
            block_time: Duration::from_secs(10),
            diff_adjust_blocks: 16,
            genesis_height: 0,
        }
    }
}

impl Default for ConfigP2P {
    fn default() -> Self {
        Self {
            peer_connections: 32,
            consensus: ConsensusConfigP2P::default(),
        }
    }
}
