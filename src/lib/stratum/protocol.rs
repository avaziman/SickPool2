use std::sync::Arc;

use crate::{protocol::{Protocol}, coins::coin::Coin, p2p::networking::protocol::ProtocolP2P};

use super::config::StratumConfig;

pub trait StratumProtocol: Protocol<Config = (StratumConfig, Arc<ProtocolP2P<Self::Coin>>)> {
    type Coin : Coin;
    fn fetch_new_job(&self);
}