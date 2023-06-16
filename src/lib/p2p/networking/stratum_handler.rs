use primitive_types::U256;
use serde::{de::DeserializeOwned, Serialize};

use super::protocol::{ProtocolP2P, Address};
use crate::stratum::{handler::StratumHandler, job_btc::BlockHeader, common::ShareResult};
use std::fmt::Debug;

impl<HeaderT> StratumHandler for ProtocolP2P<HeaderT>
where
    HeaderT: BlockHeader
{
    fn on_share(&self, address: Address, share_res: ShareResult) {
        // self.add_local_share(share_res)
    }
}
