use std::{sync::atomic::{AtomicUsize, Ordering}, collections::{HashMap, HashSet}};
static EXTRA_NONCE_COUNTER : AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
pub struct StratumClient {
    pub extra_nonce: usize,
    pub authorized_workers: HashSet<String>,
    pub submitted_shares : HashSet<u64>
}

impl Default for StratumClient {
    fn default() -> StratumClient{
        let extra_nonce = EXTRA_NONCE_COUNTER.load(Ordering::Relaxed);
        EXTRA_NONCE_COUNTER.store(extra_nonce + 1, Ordering::Relaxed);

        StratumClient {
            extra_nonce,
            authorized_workers: HashSet::new(),
            submitted_shares: HashSet::new()
        }
    }
}