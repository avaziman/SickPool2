use std::time::Duration;

use bitcoin::{address::NetworkUnchecked, Network};
use crypto_bigint::U256;
use serde::{Deserialize, Serialize};

use crate::p2p::networking::config::ConsensusConfigP2P;

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
    const DONATION_ADDRESS: &'static str = "bcrt1q9ude4m7uetjdwv5ud5h6qn7740ret7sznanxch";

    const ATOMIC_UNITS: u64 = 8;
    const DIFF1: U256 =
        U256::from_be_hex("00000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");

    const DEFAULT_DAEMON_PORT: u16 = 8332;
    const DEFAULT_P2P_PORT: u16 = 18332;
    const DEFAULT_STRATUM_PORT: u16 = 28332;

    fn main_pool_consensus_config() -> ConsensusConfigP2P<Self::BlockT> {
        ConsensusConfigP2P {
            parent_pool_id: U256::ZERO,
            block_time_ms: Duration::from_secs(10).as_millis() as u64,
            diff_adjust_blocks: 16,
            genesis_block: bitcoin::blockdata::constants::genesis_block(Network::Bitcoin),
            password: None,
            target_1: Self::DIFF1,
            name: String::from("main"),
            default_port_p2p: Self::DEFAULT_P2P_PORT,
            default_port_stratum: Self::DEFAULT_STRATUM_PORT,
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

