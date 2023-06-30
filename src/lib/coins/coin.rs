use crate::{
    address::Address,
    p2p::networking::{block::Block, config::{ConsensusConfigP2P, ConfigP2P}},
    stratum::job_fetcher::BlockFetcher, config::ProtocolServerConfig,
};

// todo remove the clone.
pub trait Coin : Clone {
    type BlockT: Block;
    type Address: Address<FromScript = <Self::BlockT as Block>::Script>;
    type Fetcher: BlockFetcher<Self::BlockT>;
    const DONATION_ADDRESS: &'static str;
    const NAME: &'static str;
    const ATOMIC_UNITS: u64;
    fn main_config_p2p() -> ProtocolServerConfig<ConfigP2P>;
}
