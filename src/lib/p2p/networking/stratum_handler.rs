use std::sync::Arc;

use primitive_types::U256;
use serde::{de::DeserializeOwned, Serialize};

use super::protocol::{ProtocolP2P, Address};
use crate::stratum::{handler::StratumHandler, job_btc::BlockHeader, common::ShareResult};

impl<HeaderT> StratumHandler<HeaderT> for ProtocolP2P<HeaderT>
where
    HeaderT: BlockHeader
{
    fn on_share(&self, address: Address, share: HeaderT, share_res: ShareResult)
    {
        self.add_local_share(address, share_res)
    }
}

pub struct CompleteStrartumHandler<T> {
    p2p: Arc<ProtocolP2P<T>>
}