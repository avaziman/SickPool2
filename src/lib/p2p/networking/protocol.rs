use std::{
    collections::{HashMap, VecDeque},
    fmt::Debug,
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex, RwLock,
    },
};

use io_arc::IoArc;
use itertools::Itertools;
use log::{error, info, warn};
use mio::net::TcpStream;
use primitive_types::U256;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::net::SocketAddr;

use crate::{
    protocol::Protocol,
    server::respond,
    stratum::{common::ShareResult, job_btc::BlockHeader},
};

use super::{
    hard_config::{self, CURRENT_VERSION, OLDEST_COMPATIBLE_VERSION},
    peer::Peer,
    utils::time_now_ms,
};
use bincode::{self};

pub struct ProtocolP2P<HeaderT> {
    state: State<HeaderT>,
    pub conf: ConfigP2P,
    hello_message: Messages<HeaderT>,
    connection_count: AtomicU32,
    data_dir: String,
    local_best_shares: RwLock<HashMap<Address, U256>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Hello {
    version: u32,
    listening_port: u16,
}

impl Hello {
    pub fn new(port: u16) -> Hello {
        Self {
            version: CURRENT_VERSION,
            listening_port: port,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub enum Messages<HeaderT> {
    Reject,
    Hello(Hello),
    VerAck,
    // get all the submitted shares during the current window
    GetShares,
    Shares(VecDeque<Share<HeaderT>>),
    ShareSubmit(HeaderT),
}

pub type Address = String;
pub type Reward = u64;

pub struct ShareWindow<HeaderT> {
    window: VecDeque<Share<HeaderT>>,
    sum: u64,
}
pub static DIFF1: U256 = U256::zero();

fn get_diff(hash: &U256) -> u64 {
    (DIFF1 / hash).as_u64()
}

impl<HeaderT: BlockHeader> ShareWindow<HeaderT> {
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

pub struct State<HeaderT> {
    rewards: Vec<(Address, Reward)>,
    window_shares: ShareWindow<HeaderT>,
    adresses_map: HashMap<Address, AddressInfo>,
}

impl<HeaderT: BlockHeader> State<HeaderT> {
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Share<HeaderT> {
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

impl<HeaderT: BlockHeader> Protocol for ProtocolP2P<HeaderT> {
    type Request = Vec<u8>;
    type Response = Vec<u8>;
    type Config = (ConfigP2P, String, u16); // data dir, listening port
    type ClientContext = Peer;
    type ProcessingContext = ();

    fn new(conf: Self::Config) -> Self {
        Self {
            state: State::new(),
            connection_count: AtomicU32::new(0),
            conf: conf.0,
            data_dir: conf.1,
            hello_message: Messages::Hello(Hello::new(conf.2)),
            local_best_shares: RwLock::new(HashMap::new()),
        }
    }

    fn process_request(
        &self,
        req: Self::Request,
        ctx: Arc<Mutex<Self::ClientContext>>,
        ptx: &mut Self::ProcessingContext,
    ) -> Self::Response {
        Self::serialize_message(&match Self::parse_request(&req) {
            Ok(message) => match self.process_message(ctx, message) {
                Some(k) => {
                    info!("Responded with message: {:?}", &k);
                    k
                }
                None => return Vec::new(),
            },
            Err(e) => {
                warn!("Failed to parse request: {}", e);
                Messages::Reject
            }
        })
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

    fn client_conncted(&self, stream: IoArc<TcpStream>, ctx: Arc<Mutex<Self::ClientContext>>) {
        info!("Sent hello to: {}", ctx.lock().unwrap().address);
        Self::send_message(&self.hello_message, stream);
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

impl<HeaderT: BlockHeader> ProtocolP2P<HeaderT> {
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
    pub fn parse_request(req: &[u8]) -> Result<Messages<HeaderT>, bincode::Error> {
        bincode::deserialize(&req)
    }

    fn process_message(
        &self,
        ctx: Arc<Mutex<Peer>>,
        message: Messages<HeaderT>,
    ) -> Option<Messages<HeaderT>> {
        info!("Received p2p message: {:?}", &message);
        match message {
            Messages::Hello(v) => {
                if v.version >= OLDEST_COMPATIBLE_VERSION {
                    let mut lock = ctx.lock().unwrap();
                    lock.authorized = Some(v.version);
                    lock.listening_port = Some(v.listening_port);
                    lock.save(self.get_peer_path(lock.address).as_path());
                    Some(Messages::VerAck)
                } else {
                    Some(Messages::Reject)
                }
            }
            Messages::VerAck => {
                // listening port is already known as it was used to connect...
                let mut lock = ctx.lock().unwrap();
                lock.authorized = Some(CURRENT_VERSION);
                lock.save(self.get_peer_path(lock.address).as_path());
                None
            }
            Messages::GetShares => Some(Messages::Shares(self.state.window_shares.get_shares())),
            Messages::Shares(shares) => todo!(),
            Messages::ShareSubmit(_) => todo!(),
            Messages::Reject => todo!(),
        }
    }

    pub fn send_message(message: &Messages<HeaderT>, stream: IoArc<TcpStream>) {
        respond(stream, Self::serialize_message(message).as_ref())
    }

    pub fn serialize_message(message: &Messages<HeaderT>) -> Vec<u8> {
        let mut bytes = bincode::serialize(message).unwrap();
        bytes.push('\n' as u8);
        bytes

        // can be done more elgantly with a custom buffer reader...
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
                        if peer.listening_port.is_some() {
                            peers.push(peer);
                            if peers.len() >= amount as usize {
                                break;
                            }
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
        let peers = peers
            .iter()
            .map(|p| SocketAddr::new(p.address.ip(), p.listening_port.unwrap()))
            .collect_vec();

        if peers.len() < amount as usize {
            // error!("Failed to get enough peers");
        }
        peers
    }

    fn get_peer_path(&self, address: SocketAddr) -> PathBuf {
        PathBuf::from(get_peer_path(address, &self.data_dir))
    }

    pub fn add_local_share(
        &self,
        address: Address,
        share_res: crate::stratum::common::ShareResult,
    ) {
        let lock = self.local_best_shares.read().unwrap();
        let current_best = &lock[&address];

        match share_res {
            ShareResult::Valid(hash) | ShareResult::Block(hash) => {
                if hash < *current_best {
                    info!("New best share for {}", address);
                    
                    let mut lock = self.local_best_shares.write().unwrap();
                    lock.insert(address, hash);
                }
            }
            ShareResult::Stale() => todo!(),
            ShareResult::Invalid() => todo!(),
            ShareResult::Duplicate() => todo!(),
        }
    }
}

pub fn get_peer_path(address: SocketAddr, datadir: &str) -> String {
    format!("{}/peers/{}.json", datadir, address.ip())
}
