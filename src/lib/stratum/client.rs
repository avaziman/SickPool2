use std::collections::HashSet;

use io_arc::IoArc;
use mio::net::TcpStream;
use primitive_types::U256;

#[derive(Debug)]
pub struct StratumClient {
    pub stream: IoArc<TcpStream>,
    pub extra_nonce: usize,
    pub authorized_workers: HashSet<String>,
    pub submitted_shares: HashSet<u64>,
    pub difficulty: U256,
}

impl StratumClient {
    pub fn new(stream: IoArc<TcpStream>, id: usize) -> StratumClient {
        StratumClient {
            stream,
            extra_nonce: id,
            difficulty: U256::zero(),
            authorized_workers: HashSet::new(),
            submitted_shares: HashSet::new(),
        }
    }
}
