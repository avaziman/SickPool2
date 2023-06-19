use primitive_types::U256;

use crate::p2p::networking::block::Block;

use super::header::BlockHeader;
#[derive(Debug, Clone)]
pub struct Job<T, IdT = u32> {
    pub id: IdT,
    pub block: T,
    pub target: U256,
    pub height: u32,
}

impl<T: Block> Job<T, u32> {
    pub fn new(id: u32, block: T, height: u32) -> Self {
        let target = block.get_header().get_target();
        Job { id, block, target, height}
    }
}