use std::{
    fmt::Debug,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    path::Path,
    time::Duration,
};

use crypto_bigint::U256;

use crate::{
    address::Address,
    config::{ProtocolServerConfig, ServerConfig},
    p2p::networking::{
        block::Block,
        config::{ConfigP2P, ConsensusConfigP2P},
        protocol::ProtocolP2P,
    },
    stratum::{config::StratumConfig, job_fetcher::BlockFetcher},
};

// todo remove the clone. and debug
pub trait Coin: Clone + Debug + PartialEq {
    type BlockT: Block;
    type Address: Address<FromScript = <Self::BlockT as Block>::Script>;
    type Fetcher: BlockFetcher<Self::BlockT> + Debug;
    const DONATION_ADDRESS: &'static str;
    const NAME: &'static str;
    const ATOMIC_UNITS: u64;
    const DIFF1: U256;
    const DEFAULT_DAEMON_PORT: u16;
    const DEFAULT_P2P_PORT: u16 = Self::DEFAULT_DAEMON_PORT + 10_000;
    const DEFAULT_STRATUM_PORT: u16 = Self::DEFAULT_DAEMON_PORT + 20_000;

    fn main_pool_consensus_config() -> ConsensusConfigP2P<Self::BlockT>;
    fn main_pool_config(data_dir: Box<Path>) -> ProtocolServerConfig<ConfigP2P<Self::BlockT>> {
        ProtocolServerConfig {
            server_config: ServerConfig {
                address: SocketAddr::V4(SocketAddrV4::new(
                    Ipv4Addr::LOCALHOST,
                    Self::DEFAULT_P2P_PORT,
                )),
                processing_threads: 1,
            },
            protocol_config: ConfigP2P {
                consensus: Self::main_pool_consensus_config(),
                max_peer_connections: 16,
                rpc_url: format!("http://127.0.0.1:{}", Self::DEFAULT_DAEMON_PORT),
                data_dir, /* : Path::new(&format!("./data/{}", Self::NAME)).into() */
                listening_port: Self::DEFAULT_P2P_PORT,
            },
        }
    }
    fn default_stratum_config() -> ProtocolServerConfig<StratumConfig> {
        ProtocolServerConfig {
            server_config: ServerConfig {
                address: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, Self::DEFAULT_STRATUM_PORT)),
                processing_threads: 2,
            },
            protocol_config: StratumConfig {
                rpc_url: format!("http://127.0.0.1:{}", Self::DEFAULT_DAEMON_PORT),
                job_poll_interval_ms: Duration::from_secs(1).as_millis() as u64,
                default_diff_units: 10000,
            },
        }
    }
}

// impl<C: Coin> Default for ConfigP2P<C> {
//     fn default() -> Self {
//         Self {
//             max_peer_connections: todo!(),
//             rpc_url: todo!(),
//             data_dir: Path::new(&format!("./data/{}", C::NAME)).into(),
//             consensus: C::main_pool_consensus_config(),
//             listening_port: todo!(),
//         }
//     }
// }
