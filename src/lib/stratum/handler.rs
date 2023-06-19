use primitive_types::U256;

use crate::p2p::networking::{protocol::Address, block::Block};

use super::{common::ShareResult, header::BlockHeader};

pub trait StratumHandler<BlockT: Block> {
    fn on_valid_share(&self, address: Address, share: &BlockT, hash: U256);
    fn on_new_block(&self, height: u32, header: &BlockT){}
}
