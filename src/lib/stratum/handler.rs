use crypto_bigint::U256;

use crate::{coins::coin::Coin, p2p::networking::block::Block};

use super::job_fetcher::BlockFetcher;

pub trait StratumHandler<C: Coin> {
    fn on_valid_share(
        &self,
        address: &C::Address,
        share: &C::BlockT,
        hash: U256,
    );
    fn on_new_block(&self, height: u32, header: &C::BlockT);
}
