use std::sync::{Arc, atomic::Ordering};

use log::info;
use primitive_types::U256;
use serde::{de::DeserializeOwned, Serialize};

use super::{
    block::Block,
    protocol::{Address, Messages, ProtocolP2P, ShareP2P},
};
use crate::stratum::{common::ShareResult, handler::StratumHandler, header::BlockHeader};

impl<BlockT> StratumHandler<BlockT> for ProtocolP2P<BlockT>
where
    BlockT: Block,
{
    fn on_valid_share(&self, address: Address, share: &BlockT, hash: U256) {
    }

    fn on_new_block(&self, height: u32, header: &BlockT) {
        self.current_height.store(height, Ordering::Relaxed);
        
        let peer_lock = self.peers.lock().unwrap();

        // for (addr, (share, diff)) in &*lock {
        //     let share = ShareP2P {
        //         address: addr.clone(),
        //         block: share.clone(),
        //     };
        //     let message = Messages::ShareSubmit(share);
        //     for (token, stream) in &*peer_lock {
        //         Self::send_message(&message, stream.clone());
        //         info!("Submitted share: {:?} for address {}", message, addr);
        //     }
        // }
    }
}

pub struct CompleteStrartumHandler<T> {
    pub p2p: Arc<ProtocolP2P<T>>,
}

impl<HeaderT> StratumHandler<HeaderT> for CompleteStrartumHandler<HeaderT>
where
    HeaderT: Block,
{
    fn on_valid_share(&self, address: Address, share: &HeaderT, hash: U256) {
        self.p2p.on_valid_share(address, share, hash)
    }

    fn on_new_block(&self, height: u32, header: &HeaderT) {
        self.p2p.on_new_block(height, header)
    }
}
