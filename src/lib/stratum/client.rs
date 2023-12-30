use std::{collections::{HashSet, HashMap}, net::SocketAddr};



use crypto_bigint::U256;

use crate::{server::Notifier, p2p::duplicate_checker::DuplicateHashChecker};

#[derive(Debug)]
pub struct StratumClient {
    pub address: SocketAddr,
    pub notifier: Notifier,
    pub extra_nonce: u32,
    pub authorized_workers: HashMap<String, String>,
    pub submitted_shares: DuplicateHashChecker,
    pub target: U256,
    pub subscription_key: Option<usize>
}

impl StratumClient {
    pub fn new(notifier: Notifier, id: u32, address: SocketAddr) -> StratumClient {
        StratumClient {
            notifier,
            extra_nonce: id,
            target: U256::ZERO,
            authorized_workers: HashMap::new(),
            submitted_shares: DuplicateHashChecker::default(),
            subscription_key: None,
            address,
        }
    }
}
