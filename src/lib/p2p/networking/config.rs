use std::time::Duration;

use crypto_bigint::U256;
use duration_str::deserialize_duration;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigP2P {
    pub peer_connections: u32,
    pub rpc_url: String,
    #[serde(flatten)]
    pub consensus: ConsensusConfigP2P,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConsensusConfigP2P {
    // #[serde(with = "SerHex::<Strict>")]
    pub genesis_block_hash: U256,
    #[serde(deserialize_with = "deserialize_duration")]
    pub block_time: Duration,
    pub diff_adjust_blocks: u32,
}
