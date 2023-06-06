use bitcoincore_rpc::{bitcoin::Target};

use super::job_btc::BlockHeader;
#[derive(Debug)]

pub struct Job<T: BlockHeader, IdT = u32> {
    pub id: IdT,
    pub header: T,
    pub target: Target,
}

impl<T: BlockHeader> Job<T, u32> {
    pub fn new(id: u32, header: T) -> Self {
        let target = header.get_target();
        Job { id, header, target}
    }
}