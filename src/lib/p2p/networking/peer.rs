use serde::{Deserialize, Serialize};
use std::{net::SocketAddr};

type UnixMs = u64;

#[derive(Debug, Deserialize, Serialize)]
pub struct Peer {
    pub address: SocketAddr,
    // acknowledged version
    pub last_connection_fail: Option<UnixMs>,

    // runtime variables
    #[serde(skip)]
    pub authorized: Option<u32>,
    pub listening_port: Option<u16>,

    #[serde(default = "bool::default")]
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub connected: bool,
}

impl Peer {
    pub fn new(address: SocketAddr) -> Self {
        Peer {
            address,
            last_connection_fail: None,
            authorized: None,
            listening_port: None,
            connected: true,
        }
    }
}
