use std::time::Duration;

use crypto_bigint::{U256, Encoding};
use duration_str::deserialize_duration;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};


use super::block::Block;

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigP2P<BlockT> {
    pub peer_connections: u32,
    pub rpc_url: String,
    #[serde(flatten)]
    pub consensus: ConsensusConfigP2P<BlockT>,
}

#[derive(Serialize, Deserialize, Debug)]
// pool id is the consensus hash
pub struct ConsensusConfigP2P<BlockT> {
    // zero for main pool
    pub parent_pool_id: U256,
    pub target_1: U256,
    pub password: Option<String>,
    // genesis pool block also gives us the main height of start
    pub genesis_share: BlockT,
    #[serde(deserialize_with = "deserialize_duration")]
    pub block_time: Duration,
    pub diff_adjust_blocks: u32,
}
// pools that havent submitted shares in a week should be remove from explorable
// pool target must be below the target of its ancestors

impl<BlockT: Block> ConsensusConfigP2P<BlockT> {
    pub fn pool_id(&self) -> U256 {
        U256::from_le_bytes(Sha256::digest(&bincode::serialize(&self).unwrap()).into())
    }
}