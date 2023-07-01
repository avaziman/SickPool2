use std::fmt::Debug;

use crypto_bigint::U256;

use crate::{
    address::Address,
    p2p::networking::{block::Block, config::{ConfigP2P}},
    stratum::job_fetcher::BlockFetcher, config::ProtocolServerConfig,
};

// todo remove the clone. and debug
pub trait Coin : Clone + Debug + PartialEq {
    type BlockT: Block;
    type Address: Address<FromScript = <Self::BlockT as Block>::Script>;
    type Fetcher: BlockFetcher<Self::BlockT>;
    const DONATION_ADDRESS: &'static str;
    const NAME: &'static str;
    const ATOMIC_UNITS: u64;
    const DIFF1: U256;
    fn main_config_p2p() -> ProtocolServerConfig<ConfigP2P>;
}
