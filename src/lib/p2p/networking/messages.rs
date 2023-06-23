use serde::{Deserialize, Serialize};

use crate::config::ProtocolServerConfig;

use super::{config::ConfigP2P, hard_config::CURRENT_VERSION};

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
    CreatePool(ProtocolServerConfig<ConfigP2P>),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Hello {
    pub version: u32,
    pub listening_port: u16,
}

impl Hello {
    pub fn new(port: u16) -> Hello {
        Self {
            version: CURRENT_VERSION,
            listening_port: port,
        }
    }
}


#[derive(Debug, PartialEq)]
pub enum ShareVerificationError {
    BadEncoding,
    BadTarget,
    BadRewards,
    BadLinkMain,
    BadLinkP2P,
}