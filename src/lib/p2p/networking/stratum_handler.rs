use std::sync::Arc;

use crypto_bigint::U256;
use log::{error, info};

use super::{block::Block, protocol::ProtocolP2P};
use crate::{
    coins::coin::Coin,
    stratum::{handler::StratumHandler, job_fetcher::BlockFetcher},
};

impl<C: Coin> StratumHandler<C> for ProtocolP2P<C> {
    fn on_valid_share(
        &self,
        _address: &C::Address,
        block: &C::BlockT,
        hash: U256,
    ) {
        let lock = self.target_manager.lock().unwrap();
        let target = lock.target();
        if &hash > target {
            return;
        }

        match self.block_manager.process_share(
            block.clone(),
            &lock,
            &self.pplns_window.lock().unwrap(),
        ) {
            Ok(valid_p2p_share) => {
                info!(
                    "LOCAL FOUND new share submission hash: {}",
                    &valid_p2p_share.hash
                );

                self.pplns_window.lock().unwrap().add(valid_p2p_share);
            }
            Err(e) => {
                error!("LOCAL P2P share rejected for: {:?}", e);
            }
        }
    }

    fn on_new_block(&self, height: u32, block: &C::BlockT) {
        self.block_manager.new_block(height, block);
        // let mut target_lock = self.target_manager.lock().unwrap();
        // target_lock.adjust(height, block);

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

pub struct CompleteStrartumHandler<C: Coin> {
    pub p2p: Arc<ProtocolP2P<C>>,
}

impl<C: Coin> StratumHandler<C> for CompleteStrartumHandler<C> {
    fn on_valid_share(
        &self,
        address: &C::Address,
        share: &C::BlockT,
        hash: U256,
    ) {
        self.p2p.on_valid_share(address, share, hash)
    }

    fn on_new_block(
        &self,
        height: u32,
        block: &C::BlockT,
    ) {
        self.p2p.on_new_block(height, block)
    }
}
