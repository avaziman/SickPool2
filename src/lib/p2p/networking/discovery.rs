use std::net::{IpAddr, Ipv4Addr};

use super::peer::Peer;

pub fn discover_peers() -> Vec<Peer> {
    let mut v = Vec::new();
    // v.push(Peer{ address: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 0))});

    v
}
