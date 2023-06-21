use std::{
    collections::HashMap,
    fmt::Debug,
    path::Path,
    sync::{Arc, Mutex},
};

use bitcoincore_rpc::bitcoin::{CompactTarget, PublicKey, Target};
use crypto_bigint::{Encoding, U256};
use io_arc::IoArc;
use log::{info, warn};
use mio::{net::TcpStream, Token};

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use crate::{protocol::Protocol, server::respond, stratum::header::BlockHeader};

use super::{
    block::Block,
    block_manager::BlockManager,
    hard_config::{CURRENT_VERSION, OLDEST_COMPATIBLE_VERSION},
    messages::*,
    peer::Peer,
    peer_manager::PeerManager,
    pplns::{Score, ScoreChanges, WindowPPLNS},
    utils::time_now_ms,
};
use bincode::{self};

pub struct ProtocolP2P<BlockT> {
    pub pplns_window: Mutex<WindowPPLNS<BlockT>>,
    pub addresses_map: HashMap<Address, AddressInfo>,
    pub conf: ConfigP2P,
    hello_message: Messages<BlockT>,
    pub peers: Mutex<HashMap<Token, IoArc<TcpStream>>>,
    data_dir: Box<Path>,
    pub peer_manager: PeerManager,
    pub block_manager: BlockManager<BlockT>,
}

pub type Address = PublicKey;
pub type Reward = u64;

pub struct AddressInfo {
    last_share_entry: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
// p2pool difficulty (bits) is encoded inside block generation tx
pub struct ShareP2P<BlockT> {
    pub block: BlockT,
    pub encoded: CoinabseEncodedP2P,
    // #[serde(skip)]
    // hash: U256,
    pub score_changes: ScoreChanges,
}

impl<BlockT: Block> ShareP2P<BlockT> {
    pub fn genesis() -> Self {
        let block = BlockT::genesis();
        let genesis_target = block.get_header().get_target();

        Self {
            block,
            encoded: CoinabseEncodedP2P {
                diff_bits: Target::from_le_bytes(genesis_target.to_le_bytes()).to_compact_lossy(),
                rewards: Default::default(),
                height: 0,
            },
            score_changes: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CoinabseEncodedP2P {
    rewards: Vec<(Address, Reward)>,
    pub diff_bits: CompactTarget,
    pub height: u32,
}

impl CoinabseEncodedP2P {
    fn get_target(&self) -> U256 {
        U256::from_le_bytes(Target::from_compact(self.diff_bits).to_le_bytes())
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
    type Config = (ConfigP2P, Box<Path>, u16); // data dir, listening port
    type ClientContext = Peer;
    type ProcessingContext = ();

    fn new(conf: Self::Config) -> Self {
        let data_dir = conf.1;
        Self {
            addresses_map: HashMap::new(),
            pplns_window: Mutex::new(WindowPPLNS::new()),
            peers: Mutex::new(HashMap::new()),
            conf: conf.0,
            hello_message: Messages::Hello(Hello::new(conf.2)),
            peer_manager: PeerManager::new(data_dir.clone()),
            block_manager: BlockManager::new(data_dir.clone()),
            data_dir,
        }
    }

    fn process_request(
        &self,
        req: Self::Request,
        ctx: Arc<Mutex<Self::ClientContext>>,
        _ptx: &mut Self::ProcessingContext,
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
                warn!("Failed to parse message: {}", e);
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

            Some(self.peer_manager.load_connecting_peer(address))
        }
    }

    fn client_conncted(&self, stream: IoArc<TcpStream>, ctx: Arc<Mutex<Self::ClientContext>>) {
        info!("Sent hello to: {}", ctx.lock().unwrap().address);
        Self::send_message(&self.hello_message, stream);
    }

    // TODO clean
    fn delete_client(
        &self,
        _addr: SocketAddr,
        ctx: Arc<Mutex<Self::ClientContext>>,
        token: mio::Token,
    ) {
        self.peers.lock().unwrap().remove(&token);

        let mut lock = ctx.lock().unwrap();
        lock.last_connection_fail = Some(time_now_ms());
        lock.connected = false;
        self.peer_manager.save_peer(&*lock);
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
            Messages::Shares(_shares) => todo!(),
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
            self.peer_manager.save_peer(&*lock);

            Some(Messages::VerAck)
        } else {
            Some(Messages::Reject)
        }
    }

    fn handle_ver_ack(&self, ctx: Arc<Mutex<Peer>>) -> Option<Messages<BlockT>> {
        // listening port is already known as it was used to connect...
        let mut lock = ctx.lock().unwrap();
        lock.authorized = Some(CURRENT_VERSION);
        self.peer_manager.save_peer(&*lock);
        None
    }

    fn handle_get_shares(&self) -> Option<Messages<BlockT>> {
        // listening port is already known as it was used to connect...
        let shares = self.block_manager.load_shares();

        match shares {
            Ok(k) => Some(Messages::Shares(k)),
            Err(e) => Some(Messages::Reject),
        }
    }

    fn handle_share_submit(
        &self,
        ctx: Arc<Mutex<Peer>>,
        share: BlockT,
    ) -> Option<Messages<BlockT>> {
        let tip = self.block_manager.tip();
        let height = self.block_manager.height();
        let share = match share.into_p2p(tip, height) {
            Some(s) => s,
            None => {
                warn!("Invalid p2p block provided.");
                return None;
            }
        };

        let hash = share.block.get_header().get_hash();
        if hash > tip.encoded.get_target() {
            warn!("Insufficient diffiuclty");

            return None;
        }

        // TODO: make share verifying read only and appropriate locking... rwlock
        if let Err(e) = self.pplns_window.lock().unwrap().add(share, hash, height) {
            warn!("Score changes are unbalanced...");
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
    pub fn peers_to_connect(&self, amount: u32) -> Vec<SocketAddr> {
        self.peer_manager.get_peers_to_connect(amount)
    }
}
