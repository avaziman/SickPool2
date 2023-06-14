use primitive_types::U256;

use super::job_btc::BlockHeader;
#[derive(Debug, Clone)]
pub struct Job<T, IdT = u32> {
    pub id: IdT,
    pub header: T,
    pub target: U256,
}

impl<T: BlockHeader + Clone> Job<T, u32> {
    pub fn new(id: u32, header: T) -> Self {
        let target = header.get_target();
        Job { id, header, target}
    }
}