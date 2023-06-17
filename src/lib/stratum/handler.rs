use primitive_types::U256;

use crate::p2p::networking::protocol::Address;

use super::{common::ShareResult, job_btc::BlockHeader};

pub trait StratumHandler<HeaderT: BlockHeader> {
    fn on_share(&self, address: Address, share: HeaderT, share_res: ShareResult);
}