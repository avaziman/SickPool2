use std::sync::{Arc, atomic::Ordering};


use crypto_bigint::U256;


use super::{
    block::Block,
    protocol::{Address, ProtocolP2P},
};
use crate::stratum::{handler::StratumHandler};

impl<BlockT> StratumHandler<BlockT> for ProtocolP2P<BlockT>
where
    BlockT: Block,
{
    fn on_valid_share(&self, _address: Address, _share: &BlockT, _hash: U256) {
    }

    fn on_new_block(&self, height: u32, _header: &BlockT) {
        self.block_manager.new_block(height);
        
        let _peer_lock = self.peers.lock().unwrap();

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
