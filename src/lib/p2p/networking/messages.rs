use crypto_bigint::U256;
use serde::{Deserialize, Serialize};

use crate::{config::ProtocolServerConfig, p2p::consensus::consensus::ConsensusConfigP2P};

use super::{
    block::{Block, EncodeErrorP2P},
    config::{ConfigP2P},
    hard_config::CURRENT_VERSION,
};

// node needs to know and verify where the current window started

#[derive(Serialize, Deserialize, Debug)]
pub enum Messages<BlockT> {
    Reject,

    Hello(Hello),
    VerAck,

    // from_height = 0 to get shares from
    GetShares {
        from_height: u32,
        count: u8,
    },
    Shares(Vec<BlockT>),
    ShareSubmit(BlockT),

    GetRoundInfo,
    // current height used to estimate how long to sync
    RoundInfo {
        start_height: u32,
        current_height: u32,
    },

    // provide default port for each pool, for convention, address must be (LOCALHOST)
    CreatePool(ProtocolServerConfig<ConfigP2P<BlockT>>),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Hello {
    pub version: u32,
    pub listening_port: u16,
    pub pool_consensus_hash: U256,
}

impl Hello {
    pub fn new<T: Block>(port: u16, consensus: &ConsensusConfigP2P<T>) -> Hello {
        Self {
            version: CURRENT_VERSION,
            listening_port: port,
            pool_consensus_hash: consensus.pool_hash(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ShareVerificationError {
    BadEncoding(EncodeErrorP2P),
    BadTarget,
    BadRewards,
    BadLinkMain,
    BadLinkP2P,
}
