use std::sync::Arc;

use crypto_bigint::U256;
use log::info;

use super::{
    block::Block,
    protocol::{Address, ProtocolP2P},
};
use crate::stratum::handler::StratumHandler;

impl<BlockT> StratumHandler<BlockT> for ProtocolP2P<BlockT>
where
    BlockT: Block,
{
    fn on_valid_share(&self, _address: Address, block: &BlockT, hash: U256) {
        let target = *self.target_manager.lock().unwrap().target();
        if hash > target {
            return;
        }

        if let Ok(valid_p2p_share) = self.block_manager.process_share(
            block.clone(),
            &target,
            &self.pplns_window.lock().unwrap(),
        ) {
            info!("FOUND new share submission hash: {}", &valid_p2p_share.hash);

            self.pplns_window.lock().unwrap().add(valid_p2p_share);
        }
    }

    fn on_new_block(&self, height: u32, block: &BlockT) {
        self.block_manager.new_block(height, block);
        let mut target_lock = self.target_manager.lock().unwrap();
        target_lock.adjust(height, block);

        let _peer_lock = self.peers.lock().unwrap();

        // info!("Current p2p target: {}", target_lock.target());
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
