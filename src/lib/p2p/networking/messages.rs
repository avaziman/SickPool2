use crypto_bigint::U256;
use serde::{Deserialize, Serialize};

use crate::{config::ProtocolServerConfig};

use super::{config::{ConfigP2P, ConsensusConfigP2P}, hard_config::CURRENT_VERSION, block::{EncodeErrorP2P, Block}};

#[derive(Serialize, Deserialize, Debug)]
pub enum Messages<BlockT> {
    Reject,
    Hello(Hello),
    VerAck,
    // get all the submitted shares during the current window
    GetShares,
    Shares(Vec<BlockT>),
    ShareSubmit(BlockT),

    // provide default port for each pool, for convention, address must be (LOCALHOST)
    CreatePool(ProtocolServerConfig<ConfigP2P<BlockT>>),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Hello {
    pub version: u32,
    pub listening_port: u16,
    pub consensus_hash: U256,
}

impl Hello {
    pub fn new<T: Block>(port: u16, consensus: &ConsensusConfigP2P<T>) -> Hello {
        Self {
            version: CURRENT_VERSION,
            listening_port: port,
            consensus_hash: consensus.pool_id()
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