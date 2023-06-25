use std::{
    collections::HashMap,
    fmt::Debug,
    path::Path,
    str::FromStr,
    sync::{Arc, Mutex},
};

use bitcoin::{
    address::{NetworkUnchecked},
    Network,
};
use crypto_bigint::U256;

use io_arc::IoArc;
use log::{info, warn};
use mio::{net::TcpStream, Token};

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use crate::{
    protocol::Protocol,
    server::{respond, Notifier},
};

use super::{
    block::Block,
    block_manager::BlockManager,
    config::ConfigP2P,
    hard_config::{CURRENT_VERSION, DEV_ADDRESS_BTC_STR, OLDEST_COMPATIBLE_VERSION, PPLNS_DIFF_MULTIPLIER, PPLNS_SHARE_UNITS},
    messages::*,
    peer::Peer,
    peer_manager::PeerManager,
    pplns::{MyBtcAddr, ScoreChanges, WindowPPLNS},
    target_manager::TargetManager,
    utils::time_now_ms,
};
use bincode::{self};

pub struct ProtocolP2P<BlockT> {
    pub pplns_window: Mutex<WindowPPLNS<BlockT>>,
    pub conf: ConfigP2P,
    hello_message: Messages<BlockT>,
    pub peers: Mutex<HashMap<Token, Notifier>>,
    // data_dir: Box<Path>,
    pub peer_manager: PeerManager,
    pub block_manager: BlockManager<BlockT>,
    pub target_manager: Mutex<TargetManager>,
}

pub type Address = MyBtcAddr;
pub type Reward = u64;

#[derive(Serialize, Deserialize, Clone, Debug)]
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

        Self {
            encoded: CoinabseEncodedP2P {
                prev_hash: U256::ZERO,
            },
            score_changes: ScoreChanges {
                added: Vec::from([(
                    MyBtcAddr(
                        bitcoin::Address::<NetworkUnchecked>::from_str(DEV_ADDRESS_BTC_STR)
                            .unwrap()
                            .require_network(Network::Bitcoin)
                            .unwrap(),
                    ),
                    PPLNS_SHARE_UNITS * PPLNS_DIFF_MULTIPLIER,
                )]),
                removed: Vec::new(),
            },
            block,
        }
    }
}

// p2pool difficulty (bits) is encoded inside block generation tx
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CoinabseEncodedP2P {
    pub prev_hash: U256,
}

impl CoinabseEncodedP2P {
    // fn get_target(&self) -> U256 {
    //     U256::from_le_bytes(Target::from_compact(self.diff_bits).to_le_bytes())
    // }
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
            pplns_window: Mutex::new(WindowPPLNS::new()),
            target_manager: Mutex::new(TargetManager::new::<BlockT>(
                conf.0.consensus.block_time,
                conf.0.consensus.diff_adjust_blocks,
            )),
            peers: Mutex::new(HashMap::new()),
            conf: conf.0,
            hello_message: Messages::Hello(Hello::new(conf.2)),
            peer_manager: PeerManager::new(data_dir.clone()),
            block_manager: BlockManager::new(data_dir.clone()),
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
        _notifier: Notifier,
    ) -> Option<Self::ClientContext> {
        let peer_lock = self.peers.lock().unwrap();
        let connection_count = peer_lock.len() as u32;

        if connection_count >= self.conf.peer_connections {
            None
        } else {
            // peer_lock.insert(token, notifier);

            Some(self.peer_manager.load_connecting_peer(address))
        }
    }

    fn client_conncted(&self, stream: IoArc<TcpStream>, ctx: Arc<Mutex<Self::ClientContext>>) {
        info!("Sent hello to: {}", ctx.lock().unwrap().address);
        Self::send_message(&self.hello_message, stream.as_ref());
    }

    // TODO clean
    fn delete_client(&self, ctx: Arc<Mutex<Self::ClientContext>>) {
        // TODO: CHECK
        // self.peers.lock().unwrap().remove(&token);

        let mut lock = ctx.lock().unwrap();
        lock.last_connection_fail = Some(time_now_ms());
        lock.connected = false;
        self.peer_manager.save_peer(&*lock);
    }

    fn create_ptx(&self) -> Self::ProcessingContext {}
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
            Err(_e) => Some(Messages::Reject),
        }
    }

    fn handle_share_submit(
        &self,
        ctx: Arc<Mutex<Peer>>,
        share: BlockT,
    ) -> Option<Messages<BlockT>> {
        let target = *self.target_manager.lock().unwrap().target();

        match self
            .block_manager
            .process_share(share, &target, &self.pplns_window.lock().unwrap())
        {
            Ok(pshare) => {
                // check if valid mainnet block
                // let main_target = pshare.inner.block.get_header().get_target();

                info!(
                    "Accepted new share submission from peer: {}, hash: {}",
                    ctx.lock().unwrap().address,
                    &pshare.hash
                );

                self.pplns_window.lock().unwrap().add(pshare);
            }
            Err(e) => {
                info!(
                    "Rejected share from {} for {:?}",
                    ctx.lock().unwrap().address,
                    e
                )
            }
        }

        None
    }

    pub fn send_message(message: &Messages<BlockT>, stream: &TcpStream) {
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
