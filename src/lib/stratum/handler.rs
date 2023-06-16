use primitive_types::U256;

use crate::p2p::networking::protocol::Address;

use super::common::ShareResult;

pub trait StratumHandler {
    fn on_share(&self, address: Address, share_res: ShareResult);
}