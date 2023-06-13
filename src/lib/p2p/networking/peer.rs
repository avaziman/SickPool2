use std::{net::{IpAddr, SocketAddr}, collections::HashMap};
use itertools::Itertools;

const DEFAULT_PORT : u16 = 9001;

#[derive(Debug)]
pub struct Peer {
    pub address: SocketAddr,
    // acknowledged version
    pub successfully_connected: bool,
}

// pub struct RetransmitPeers {
//     root_distance: usize, // distance from the root node
//     neighbors: Vec<Peer>,
//     children: Vec<Peer>,
//     // Maps from tvu/tvu_forwards addresses to the first node
//     // in the shuffle with the same address.
//     addrs: HashMap<SocketAddr, Pubkey>, // tvu addresses
//     frwds: HashMap<SocketAddr, Pubkey>, // tvu_forwards addresses
// }

// /// Turbine logic
// /// 1 - For the current node find out if it is in layer 1
// /// 1.1 - If yes, then broadcast to all layer 1 nodes
// ///      1 - using the layer 1 index, broadcast to all layer 2 nodes assuming you know neighborhood size
// /// 1.2 - If no, then figure out what layer the node is in and who the neighbors are and only broadcast to them
// ///      1 - also check if there are nodes in the next layer and repeat the layer 1 to layer 2 logic

// /// Returns Neighbor Nodes and Children Nodes `(neighbors, children)` for a given node based on its stake
// pub fn compute_retransmit_peers<T: Copy>(
//     fanout: usize,
//     index: usize, // Local node's index withing the nodes slice.
//     nodes: &[T],
// ) -> (Vec<T> /*neighbors*/, Vec<T> /*children*/) {
//     // 1st layer: fanout    nodes starting at 0
//     // 2nd layer: fanout**2 nodes starting at fanout
//     // 3rd layer: fanout**3 nodes starting at fanout + fanout**2
//     // ...
//     // Each layer is divided into neighborhoods of fanout nodes each.
//     let offset = index % fanout; // Node's index within its neighborhood.
//     let anchor = index - offset; // First node in the neighborhood.
//     let neighbors = (anchor..)
//         .take(fanout)
//         .map(|i| nodes.get(i).copied())
//         .while_some()
//         .collect();
//     let children = ((anchor + 1) * fanout + offset..)
//         .step_by(fanout)
//         .take(fanout)
//         .map(|i| nodes.get(i).copied())
//         .while_some()
//         .collect();
//     (neighbors, children)
// }