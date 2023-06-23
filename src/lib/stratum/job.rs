use crypto_bigint::U256;
use serde_json::json;

use crate::{
    p2p::networking::block::Block,
    sickrpc::{ResultOrErr, RpcReqBody, RpcRequest},
};

use super::header::BlockHeader;
#[derive(Debug, Clone)]
pub struct Job<T, IdT = u32> {
    pub id: IdT,
    pub block: T,
    pub target: U256,
    pub height: u32,
    pub reward: u64,
}

impl<T: Block> Job<T> {
    pub fn new(id: u32, block: T, height: u32, reward: u64) -> Self {
        let target = block.get_header().get_target();
        Job {
            id,
            block,
            target,
            height,
            reward,
        }
    }
}

impl Job<bitcoin::Block> {
    pub fn get_broadcast_message(&self) -> RpcReqBody {
        let header = self.block.get_header();

        (
            String::from("mining.notify"),
            json!([
                hex::encode(self.id.to_be_bytes()),
                header.get_prev().to_string(),
                "cb1",
                "cb2",
                "mrkl",
                hex::encode(header.version.to_consensus().to_be_bytes()),
                hex::encode(header.bits.to_consensus().to_be_bytes()),
                hex::encode(header.time.to_be_bytes()),
                "true"
            ]),
        )
    }
}

// TODO: make job copy sturct for only the header
