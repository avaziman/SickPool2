use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use bitcoin::{address::NetworkUnchecked, Network};
use crypto_bigint::U256;
use serde::{Deserialize, Serialize};

use crate::{
    config::{ProtocolServerConfig, ServerConfig},
    p2p::networking::{
        config::{ConfigP2P, ConsensusConfigP2P},
        hard_config::DEFAULT_STRATUM_PORT,
    },
};

use super::coin::Coin;

#[derive(Serialize, Clone, Debug, Eq, Hash, PartialEq)]
pub struct MyBtcAddr(pub bitcoin::Address);

#[derive(Clone, Debug, PartialEq)]
pub struct Btc;

impl Coin for Btc {
    type Address = MyBtcAddr;
    type BlockT = bitcoin::Block;
    type Fetcher = bitcoincore_rpc::Client;

    const NAME: &'static str = "Bitcoin";
    const DONATION_ADDRESS: &'static str = &"bcrt1q9ude4m7uetjdwv5ud5h6qn7740ret7sznanxch";

    const ATOMIC_UNITS: u64 = 8;
    const DIFF1: U256 = U256::from_be_hex("00000000FFFF0000000000000000000000000000000000000000000000000000");

    fn main_config_p2p() -> ProtocolServerConfig<ConfigP2P> {
        ProtocolServerConfig {
            protocol_config: ConfigP2P {
                consensus: ConsensusConfigP2P {
                    block_time: Duration::from_secs(10),
                    diff_adjust_blocks: 16,
                    genesis_block_hash: U256::from_be_hex("3a052ae3c5e1684d6648b2674ccf26f65d27f489f536a015146e986ee45ffbf6"),
                },
                peer_connections: 32,
                rpc_url: String::from(""),
            },
            server_config: ServerConfig {
                address: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), DEFAULT_STRATUM_PORT),
                processing_threads: 2,
            },
        }
    }
}

impl Btc {
    pub const NETWORK: Network = Network::Regtest;
}
impl<'de> Deserialize<'de> for MyBtcAddr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(MyBtcAddr(
            bitcoin::Address::<NetworkUnchecked>::deserialize(deserializer)?
                .require_network(Btc::NETWORK)
                .unwrap(),
        ))
    }
}
