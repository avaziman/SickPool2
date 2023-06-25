use std::collections::{HashSet, HashMap};



use crypto_bigint::U256;

use crate::{p2p::networking::protocol::Address, server::Notifier};

#[derive(Debug)]
pub struct StratumClient {
    pub notifier: Notifier,
    pub extra_nonce: u32,
    pub authorized_workers: HashMap<String, Address>,
    pub submitted_shares: HashSet<u64>,
    pub target: U256,
    pub subscription_key: Option<usize>
}

impl StratumClient {
    pub fn new(notifier: Notifier, id: u32) -> StratumClient {
        StratumClient {
            notifier,
            extra_nonce: id,
            target: U256::ZERO,
            authorized_workers: HashMap::new(),
            submitted_shares: HashSet::new(),
            subscription_key: None
        }
    }
}
