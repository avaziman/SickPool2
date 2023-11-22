use std::sync::{Arc, Mutex};

use crypto_bigint::U256;
use log::{error, info};

use super::protocol::ProtocolP2P;
use crate::{
    coins::coin::Coin,
    p2p::networking::protocol::SubmittingContext,
    stratum::{client::StratumClient, handler::StratumHandler},
};

impl<C: Coin> StratumHandler<C> for ProtocolP2P<C> {
    fn on_valid_share(
        &self,
        ctx: Arc<Mutex<StratumClient>>,
        _address: &C::Address,
        block: &C::BlockT,
        hash: U256,
    ) {
        let lock = self.target_manager.lock().unwrap();
        let target = lock.target();
        log::info!("Hash {:x}", hash);
        log::info!("Target {:x}", target);

        if &hash > target {
            return;
        }
        std::mem::drop(lock);

        info!("LOCAL FOUND new share submission hash: {}", &hash);

        self.handle_share_submit(
            SubmittingContext::Stratum(ctx.lock().unwrap().address),
            block.clone(),
        );
    }

    fn on_new_block(&self, height: u32, block_hash: &U256) {
        self.block_manager.new_block(height, block_hash);
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

// todo add notifier
pub struct CompleteStratumHandler<C: Coin> {
    pub p2p: Arc<ProtocolP2P<C>>,
}

impl<C: Coin> StratumHandler<C> for CompleteStratumHandler<C> {
    fn on_valid_share(
        &self,
        ctx: Arc<Mutex<StratumClient>>,
        address: &C::Address,
        share: &C::BlockT,
        hash: U256,
    ) {
        self.p2p.on_valid_share(ctx, address, share, hash)
    }

    fn on_new_block(&self, height: u32, block_hash: &U256) {
        self.p2p.on_new_block(height, block_hash)
    }
}
