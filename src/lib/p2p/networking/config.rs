use std::{time::Duration, path::Path};

use crypto_bigint::{U256, Encoding};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};


use crate::p2p::consensus::consensus::ConsensusConfigP2P;

use super::block::Block;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigP2P<BlockT> {
    pub consensus: ConsensusConfigP2P<BlockT>,
    pub max_peer_connections: u32,
    pub rpc_url: String,
    // #[serde(flatten)]
    pub data_dir: Box<Path>,
    // needs to be aware of his own listening for the protocol
    pub listening_port: u16,
}

