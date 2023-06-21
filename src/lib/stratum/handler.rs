use crypto_bigint::U256;

use crate::p2p::networking::{protocol::Address, block::Block};



pub trait StratumHandler<BlockT: Block> {
    fn on_valid_share(&self, address: Address, share: &BlockT, hash: U256);
    fn on_new_block(&self, _height: u32, _header: &BlockT){}
}
