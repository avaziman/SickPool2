use std::sync::{Mutex, Arc};

use crypto_bigint::U256;

use crate::{coins::coin::Coin};

use super::client::StratumClient;


pub trait StratumHandler<C: Coin> {
    fn on_valid_share(
        &self,
        ctx: Arc<Mutex<StratumClient>>,
        address: &C::Address,
        share: &C::BlockT,
        hash: U256,
    );
    fn on_new_block(&self, height: u32, block_hash: &U256);
}