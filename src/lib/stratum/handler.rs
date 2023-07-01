use crypto_bigint::U256;

use crate::{coins::coin::Coin};



pub trait StratumHandler<C: Coin> {
    fn on_valid_share(
        &self,
        address: &C::Address,
        share: &C::BlockT,
        hash: U256,
    );
    fn on_new_block(&self, height: u32, block_hash: &U256);
}
