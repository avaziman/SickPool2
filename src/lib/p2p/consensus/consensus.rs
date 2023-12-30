use crypto_bigint::{U256, Encoding};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::p2p::networking::block::Block;

#[derive(Serialize, Deserialize, Debug, Clone)]
// pool hash is the consensus hash
pub struct ConsensusConfigP2P<BlockT> {
    pub name: String,
    // zero for main pool
    pub parent_pool_hash: U256,
    pub target_1: U256,
    pub password: Option<String>,
    // genesis pool block also gives us the main height of start
    pub genesis_block: BlockT,
    pub diff_adjust_blocks: u32,
    pub block_time_ms: u32,
    pub default_port_p2p: u16,
    pub default_port_stratum: u16,
}
// pools that havent submitted shares in a week should be remove from explorable
// pool target must be easier than the target of its ancestors

impl<BlockT: Block> ConsensusConfigP2P<BlockT> {
    pub fn pool_hash(&self) -> U256 {
        U256::from_le_bytes(Sha256::digest(&bincode::serialize(&self).unwrap()).into())
    }
}