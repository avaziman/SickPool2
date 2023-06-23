use std::{
    fs,
    net::{IpAddr, SocketAddr},
    path::Path,
};

use itertools::Itertools;

use super::{peer::Peer, utils::time_now_ms};

pub struct PeerManager {
    peers_dir: Box<Path>,
}

impl PeerManager {
    pub fn new(data_dir: Box<Path>) -> Self {
        let mut buf = data_dir.into_path_buf();
        buf.push("peers");
        let peers_dir = buf.into_boxed_path();

        Self { peers_dir }
    }

    pub fn load_peer(&self, addr: IpAddr) -> std::io::Result<Peer> {
        let path = self.get_peer_path(addr);
        Ok(serde_json::from_slice(&fs::read(&path)?)
            .expect(&format!("Bad peer file at: {}", path.display())))
    }

    pub fn load_connecting_peer(&self, address: SocketAddr) -> Peer {
        let peer = match self.load_peer(address.ip()) {
            Ok(mut exists) => {
                // we are about to connect... this method is called on connection.
                exists.connected = true;
                exists
            }
            Err(_e) => {
                let newp = Peer::new(address);

                newp
            }
        };
        self.save_peer(&peer);
        peer
    }

    pub fn save_peer(&self, peer: &Peer) {
        let path = self.get_peer_path(peer.address.ip());

        fs::write(path, serde_json::to_string_pretty(peer).unwrap()).unwrap();
    }

    pub fn get_peers_to_connect(&self, amount: u32) -> Vec<SocketAddr> {
        let mut peers: Vec<Peer> = Vec::with_capacity(amount as usize);
        match fs::read_dir(&self.peers_dir) {
            Ok(s) => {
                for f in s {
                    let f = f.unwrap();
                    let peer = self
                        .load_peer(
                            f.path()
                                .file_stem()
                                .unwrap()
                                .to_os_string()
                                .into_string()
                                .unwrap()
                                .as_str()
                                .parse()
                                .unwrap(),
                        )
                        .expect("Bad peer file");
                    // only try to connect to a peer once every ...
                    let reconnection_cooldown: u64 = 10 * 1000;

                    if !peer.connected
                        && time_now_ms() - peer.last_connection_fail.unwrap_or_default()
                            > reconnection_cooldown
                    {
                        if peer.listening_port.is_some() {
                            peers.push(peer);
                            if peers.len() >= amount as usize {
                                break;
                            }
                        }
                    }
                }
            }
            Err(_e) => panic!("No peer list!"),
        };

        let peers = peers
            .iter()
            .map(|p| SocketAddr::new(p.address.ip(), p.listening_port.unwrap()))
            .collect_vec();

        if peers.len() < amount as usize {
            // error!("Failed to get enough peers");
        }
        peers
    }

    fn get_peer_path(&self, address: IpAddr) -> Box<Path> {
        let mut path = self.peers_dir.to_path_buf();
        path.push(address.to_string() + ".json");
        path.set_extension("json");
        path.into_boxed_path()
    }
}
