use std::{
    collections::{HashMap, VecDeque},
    fmt::Debug,
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex,
    },
};

use bitcoincore_rpc::bitcoin::{CompactTarget, Target};
use io_arc::IoArc;
use itertools::Itertools;
use log::{error, info, warn};
use mio::{net::TcpStream, Token};
use primitive_types::U256;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use crate::{
    config::ProtocolServerConfig, protocol::Protocol, server::respond, stratum::header::BlockHeader,
};

use super::{
    block::Block,
    hard_config::{CURRENT_VERSION, OLDEST_COMPATIBLE_VERSION},
    peer::Peer,
    utils::time_now_ms,
};
use bincode::{self};

pub struct ProtocolP2P<BlockT> {
    pub state: Mutex<State<BlockT>>,
    chain_tip: ShareP2P<BlockT>,
    pub conf: ConfigP2P,
    hello_message: Messages<BlockT>,
    pub peers: Mutex<HashMap<Token, IoArc<TcpStream>>>,
    data_dir: String,
    pub current_height: AtomicU32,
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
pub enum Messages<BlockT> {
    Reject,
    Hello(Hello),
    VerAck,
    // get all the submitted shares during the current window
    GetShares,
    Shares(Vec<BlockT>),
    ShareSubmit(BlockT),

    // provide default port for each pool, for convention, address must be (LOCALHOST)
    CreatePool(ProtocolServerConfig<ConfigP2P>),
}

pub type Address = String;
pub type Reward = u64;

pub struct ShareWindow<HeaderT> {
    window: VecDeque<(ShareP2P<HeaderT>, U256, u32)>, // hash, height
    sum: u64,
}
pub static DIFF1: U256 = U256::zero();

fn get_diff(hash: &U256) -> u64 {
    (DIFF1 / hash).as_u64()
}

impl<BlockT> ShareWindow<BlockT>
where
    BlockT: Block,
{
    fn new() -> Self {
        Self {
            window: VecDeque::new(),
            sum: 0u64,
        }
    }

    fn add(&mut self, share: ShareP2P<BlockT>, hash: U256, height: u32) {
        self.sum += get_diff(&hash);
        self.window.push_front((share, hash, height));
        // TODO: clean old
    }

    pub fn get_reward(&self, i: usize, reward: Reward) -> u64 {
        let (share, hash, _height) = &self.window[i];

        (reward * get_diff(hash)) / self.sum
    }

    // pub fn clean_expired(&mut self, current_height: u32) {
    //     // while let Some(back) = self.window.back() {
    //     //     if !is_eligble_to_submit(back.2, current_height) {
    //     //         self.window.pop_back();
    //     //     } else {
    //     //         break;
    //     //     }
    //     // }
    // }

    fn get_shares(&self) -> VecDeque<(ShareP2P<BlockT>, U256, u32)> {
        self.window.clone()
    }
}

pub struct State<HeaderT> {
    rewards: Vec<(Address, Reward)>,
    pub window_shares: ShareWindow<HeaderT>,
    pub addresses_map: HashMap<Address, AddressInfo>,
}

impl<BlockT: Block> State<BlockT> {
    pub fn new() -> Self {
        Self {
            rewards: Vec::new(),
            window_shares: ShareWindow::new(),
            addresses_map: HashMap::new(),
        }
    }
}

pub struct AddressInfo {
    last_submit: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
// p2pool difficulty (bits) is encoded inside block generation tx
pub struct ShareP2P<BlockT> {
    pub block: BlockT,
    pub encoded: CoinabseEncodedP2P,
    // #[serde(skip)]
    // hash: U256,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CoinabseEncodedP2P {
    pub diff_bits: CompactTarget,
}

impl CoinabseEncodedP2P {
    fn get_target(&self) -> U256 {
        U256::from(Target::from_compact(self.diff_bits).to_le_bytes())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigP2P {
    pub peer_connections: u32,
    pub consensus: ConsensusConfigP2P,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConsensusConfigP2P {
    pub difficulty_adjust: u32,
}

impl Default for ConsensusConfigP2P {
    fn default() -> Self {
        Self {
            difficulty_adjust: 10,
        }
    }
}

impl Default for ConfigP2P {
    fn default() -> Self {
        Self {
            peer_connections: 32,
            consensus: ConsensusConfigP2P::default(),
        }
    }
}

impl<BlockT: Block> Protocol for ProtocolP2P<BlockT> {
    type Request = Vec<u8>;
    type Response = Vec<u8>;
    type Config = (ConfigP2P, String, u16); // data dir, listening port
    type ClientContext = Peer;
    type ProcessingContext = ();

    fn new(conf: Self::Config) -> Self {
        Self {
            state: Mutex::new(State::new()),
            peers: Mutex::new(HashMap::new()),
            conf: conf.0,
            data_dir: conf.1,
            hello_message: Messages::Hello(Hello::new(conf.2)),
            current_height: AtomicU32::new(0),
            chain_tip: todo!(),
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
        token: mio::Token,
    ) -> Option<Self::ClientContext> {
        let mut lock = self.peers.lock().unwrap();
        let connection_count = lock.len() as u32;

        if connection_count >= self.conf.peer_connections {
            None
        } else {
            lock.insert(token, stream);

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
    fn delete_client(
        &self,
        addr: SocketAddr,
        ctx: Arc<Mutex<Self::ClientContext>>,
        token: mio::Token,
    ) {
        self.peers.lock().unwrap().remove(&token);

        let mut lock = ctx.lock().unwrap();
        lock.last_connection_fail = Some(time_now_ms());
        lock.connected = false;
        lock.save(self.get_peer_path(lock.address).as_path());
    }
}

impl<BlockT: Block> ProtocolP2P<BlockT> {
    #[doc(hidden)]
    pub fn parse_request(req: &[u8]) -> Result<Messages<BlockT>, bincode::Error> {
        bincode::deserialize(&req)
    }

    fn process_message(
        &self,
        ctx: Arc<Mutex<Peer>>,
        message: Messages<BlockT>,
    ) -> Option<Messages<BlockT>> {
        info!("Received p2p message: {:?}", &message);
        match message {
            Messages::Hello(hello) => self.handle_hello(hello, ctx),
            Messages::VerAck => self.handle_ver_ack(ctx),
            Messages::GetShares => self.handle_get_shares(),
            Messages::Shares(shares) => todo!(),
            Messages::ShareSubmit(share) => self.handle_share_submit(ctx, share),
            Messages::Reject => todo!(),
            Messages::CreatePool(_) => todo!(),
        }
    }

    fn handle_hello(&self, hello: Hello, ctx: Arc<Mutex<Peer>>) -> Option<Messages<BlockT>> {
        if hello.version >= OLDEST_COMPATIBLE_VERSION {
            let mut lock = ctx.lock().unwrap();
            lock.authorized = Some(hello.version);
            lock.listening_port = Some(hello.listening_port);
            lock.save(self.get_peer_path(lock.address).as_path());
            Some(Messages::VerAck)
        } else {
            Some(Messages::Reject)
        }
    }

    fn handle_ver_ack(&self, ctx: Arc<Mutex<Peer>>) -> Option<Messages<BlockT>> {
        // listening port is already known as it was used to connect...
        let mut lock = ctx.lock().unwrap();
        lock.authorized = Some(CURRENT_VERSION);
        lock.save(self.get_peer_path(lock.address).as_path());
        None
    }

    fn handle_get_shares(&self) -> Option<Messages<BlockT>> {
        // listening port is already known as it was used to connect...
        Some(Messages::Shares(
            self.state
                .lock()
                .unwrap()
                .window_shares
                .get_shares()
                .iter()
                .map(|s| s.0.block.clone())
                .collect_vec(),
        ))
    }

    fn handle_share_submit(
        &self,
        ctx: Arc<Mutex<Peer>>,
        share: BlockT,
    ) -> Option<Messages<BlockT>> {
        let current_height = self.current_height.load(Ordering::Relaxed);
        let share = match share.into_p2p(current_height) {
            Some(s) => s,
            None => {
                warn!("Invalid p2p block provided.");
                return None;
            }
        };

        let hash = share.block.get_header().get_hash();
        if hash > share.encoded.get_target() {
            warn!("Insufficient diffiuclty");

            return None;
        }

        if !share
            .block
            .verify_coinbase_rewards(&self.state.lock().unwrap().window_shares)
        {
            warn!(
                "Received bad share submission, peer: {}",
                ctx.lock().unwrap().address
            );
            return None;
        }

        info!(
            "Accepted new share submission from peer: {}, hash: {}",
            ctx.lock().unwrap().address,
            &hash
        );

        None
    }

    pub fn send_message(message: &Messages<BlockT>, stream: IoArc<TcpStream>) {
        respond(stream, Self::serialize_message(message).as_ref())
    }

    pub fn serialize_message(message: &Messages<BlockT>) -> Vec<u8> {
        let mut bytes = bincode::serialize(message).unwrap();
        bytes.push('\n' as u8);
        bytes

        // can be done more elgantly with a custom buffer reader... or can it?
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
}

pub fn get_peer_path(address: SocketAddr, datadir: &str) -> String {
    format!("{}/peers/{}.json", datadir, address.ip())
}
