use std::{
    collections::{HashMap, VecDeque},
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex,
    },
};

use io_arc::IoArc;
use itertools::Itertools;
use log::{error, info, warn};
use mio::net::TcpStream;
use primitive_types::U256;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::net::SocketAddr;

use crate::{protocol::Protocol, stratum::job_btc::BlockHeader};

use super::{
    hard_config::{self, OLDEST_COMPATIBLE_VERSION},
    peer::Peer,
    utils::time_now_ms,
};
use bincode::{self};

pub struct ProtocolP2P<HeaderT: BlockHeader + DeserializeOwned + Serialize + Clone> {
    state: State<HeaderT>,
    pub conf: ConfigP2P,
    connection_count: AtomicU32,
    data_dir: String,
}

#[derive(Deserialize, Serialize)]
pub enum Messages<HeaderT: BlockHeader + Clone> {
    Reject,
    Version(u32),
    VerAck,
    // get all the submitted shares during the current window
    GetShares,
    Shares(VecDeque<Share<HeaderT>>),
    ShareSubmit(HeaderT),
}

pub type Address = u8;
pub type Reward = u64;

pub struct ShareWindow<HeaderT: BlockHeader + Clone> {
    window: VecDeque<Share<HeaderT>>,
    sum: u64,
}
pub static DIFF1: U256 = U256::zero();

fn get_diff(hash: &U256) -> u64 {
    (DIFF1 / hash).as_u64()
}

impl<HeaderT: BlockHeader + Clone> ShareWindow<HeaderT> {
    fn new() -> Self {
        Self {
            window: VecDeque::with_capacity(hard_config::BLOCK_WINDOW as usize),
            sum: 0u64,
        }
    }

    fn add(&mut self, share: Share<HeaderT>) {
        self.sum += get_diff(&share.hash);
        self.window.push_front(share);
    }

    pub fn get_reward(&self, i: usize, reward: Reward) -> u64 {
        let share = &self.window[i];

        (reward * get_diff(&share.hash)) / self.sum
    }

    fn clean_expired(&mut self, current_height: u32) {
        while let Some(back) = self.window.back() {
            if !is_eligble_to_submit(back.submit_height, current_height) {
                self.window.pop_back();
            } else {
                break;
            }
        }
    }

    fn get_shares(&self) -> VecDeque<Share<HeaderT>> {
        self.window.clone()
    }
}

pub struct State<HeaderT: BlockHeader + Clone> {
    rewards: Vec<(Address, Reward)>,
    window_shares: ShareWindow<HeaderT>,
    adresses_map: HashMap<Address, AddressInfo>,
}

impl<HeaderT: BlockHeader + Clone> State<HeaderT> {
    pub fn new() -> Self {
        Self {
            rewards: Vec::new(),
            window_shares: ShareWindow::new(),
            adresses_map: HashMap::new(),
        }
    }
}

struct AddressInfo {
    last_submit: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Share<HeaderT: BlockHeader + Clone> {
    address: Address,
    header: HeaderT,
    submit_height: u32,

    #[serde(skip)]
    hash: U256,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigP2P {
    pub peer_connections: u32,
}

impl Default for ConfigP2P {
    fn default() -> Self {
        Self {
            peer_connections: 32,
        }
    }
}

impl<HeaderT: BlockHeader + DeserializeOwned + Serialize + Clone> Protocol
    for ProtocolP2P<HeaderT>
{
    type Request = String;
    type Response = String;
    type Config = (ConfigP2P, String); // data dir
    type ClientContext = Peer;
    type ProcessingContext = ();

    fn new(conf: Self::Config) -> Self {
        Self {
            state: State::new(),
            connection_count: AtomicU32::new(0),
            conf: conf.0,
            data_dir: conf.1,
        }
    }

    fn process_request(
        &self,
        req: Self::Request,
        ctx: Arc<Mutex<Self::ClientContext>>,
        ptx: &mut Self::ProcessingContext,
    ) -> Self::Response {
        String::from_utf8(
            bincode::serialize(&match Self::parse_request(&req) {
                Ok(message) => self.process_message(ctx, message).unwrap(),
                Err(e) => {
                    warn!("Failed to parse request: {}", e);
                    Messages::Reject
                }
            })
            .unwrap(),
        )
        .unwrap()
    }

    // TODO reject if hit limit
    fn create_client(
        &self,
        address: SocketAddr,
        stream: IoArc<TcpStream>,
    ) -> Option<Self::ClientContext> {
        let connection_count = self.connection_count.load(Ordering::Relaxed);
        if connection_count >= self.conf.peer_connections {
            None
        } else {
            self.connection_count
                .store(connection_count + 1, Ordering::Relaxed);
            Some(Self::ClientContext::new(
                &self.get_peer_path(address).as_path(),
                address,
            ))
        }
    }

    // TODO clean
    fn delete_client(&self, addr: SocketAddr, ctx: Arc<Mutex<Self::ClientContext>>) {
        let mut lock = ctx.lock().unwrap();
        lock.last_connection_fail = Some(time_now_ms());
        lock.connected = false;
        lock.save(self.get_peer_path(lock.address).as_path());
    }
}

fn is_eligble_to_submit(last_submit: u32, current_height: u32) -> bool {
    last_submit + hard_config::BLOCK_WINDOW > current_height
}

impl<HeaderT: BlockHeader + DeserializeOwned + Serialize + Clone> ProtocolP2P<HeaderT> {
    fn verify_share(&mut self, share: Share<HeaderT>) {
        let submitter_info = &self.state.adresses_map[&share.address];
        let current_height = 0;

        if !is_eligble_to_submit(submitter_info.last_submit, current_height) {
            return;
        }

        let hash = share.header.get_hash();
    }

    fn new_block(&mut self) {
        let current_height = 0;

        self.state.window_shares.clean_expired(current_height)
    }

    #[doc(hidden)]
    pub fn parse_request(req: &String) -> Result<Messages<HeaderT>, bincode::Error> {
        bincode::deserialize(&req.as_bytes())
    }

    fn process_message(
        &self,
        ctx: Arc<Mutex<Peer>>,
        message: Messages<HeaderT>,
    ) -> Option<Messages<HeaderT>> {
        match message {
            Messages::Version(v) => {
                if v >= OLDEST_COMPATIBLE_VERSION {
                    let mut lock = ctx.lock().unwrap();
                    lock.authorized = true;
                    lock.save(self.get_peer_path(lock.address).as_path());
                    Some(Messages::VerAck)
                } else {
                    Some(Messages::Reject)
                }
            }
            Messages::VerAck => {
                let mut lock = ctx.lock().unwrap();
                lock.authorized = true;
                lock.save(self.get_peer_path(lock.address).as_path());
                None
            }
            Messages::GetShares => Some(Messages::Shares(self.state.window_shares.get_shares())),
            Messages::Shares(shares) => todo!(),
            Messages::ShareSubmit(_) => todo!(),
            Messages::Reject => todo!(),
        }
    }

    // tells the server who to connect to at bootstrap
    pub fn peers_to_connect(&self, amount: u32) -> Vec<std::net::SocketAddr> {
        let mut peers: Vec<Peer> = Vec::with_capacity(amount as usize);
        match fs::read_dir(format!("{}/peers", self.data_dir)) {
            Ok(s) => {
                for f in s {
                    let f = f.unwrap();
                    let peer = Peer::load(f.path().as_path()).expect("Bad peer file");
                    // only try to connect to a peer once every ...
                    let reconnection_cooldown: u64 = 10 * 1000;

                    if !peer.connected
                        && time_now_ms() - peer.last_connection_fail.unwrap_or_default()
                            > reconnection_cooldown
                    {
                        peers.push(peer);
                        if peers.len() >= amount as usize {
                            break;
                        }
                    }
                }
            }
            Err(e) => panic!("No peer list!"),
        };

        // .unwrap_or(String::new())
        // .lines()
        // .map(|s| s.parse().expect("Invalid peers"))
        // .filter(|s| s)
        // .collect();

        // while (peers.len() as u32) < conf.peer_connections {
        //     info!("Discovering peers...");
        // }
        let peers = peers.iter().map(|p| p.address).collect_vec();

        if peers.len() < amount as usize {
            error!("Failed to get enough peers");
        }
        peers
    }

    fn get_peer_path(&self, address: SocketAddr) -> PathBuf {
        PathBuf::from(&format!("{}/peers/{}.json", self.data_dir, address))
    }
}
