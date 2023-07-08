use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

use crypto_bigint::U256;
use io_arc::IoArc;
use log::{info, warn};
use mio::{net::TcpStream, Token};
use sha2::digest::typenum::U2;

use std::net::SocketAddr;

use crate::{
    address::Address,
    protocol::Protocol,
    server::{respond, Notifier},
    stratum::job_fetcher::BlockFetcher,
};

use super::{
    block_manager::BlockManager,
    config::{ConfigP2P, ConsensusConfigP2P},
    difficulty,
    hard_config::{
        CURRENT_VERSION, DEV_ADDRESS_BTC_STR, OLDEST_COMPATIBLE_VERSION, PPLNS_DIFF_MULTIPLIER,
        PPLNS_SHARE_UNITS,
    },
    messages::*,
    peer::Peer,
    peer_manager::PeerManager,
    pplns::{WindowPPLNS, self},
    target_manager::TargetManager,
    utils::time_now_ms,
};
use crate::coins::coin::Coin;
use bincode::{self};

pub struct ProtocolP2P<C: Coin> {
    pub pplns_window: Mutex<WindowPPLNS<C>>,
    pub conf: ConfigP2P<C::BlockT>,
    hello_message: Messages<C::BlockT>,
    pub peers: Mutex<HashMap<Token, Notifier>>,
    // data_dir: Box<Path>,
    pub peer_manager: PeerManager,
    pub block_manager: BlockManager<C>,
    pub target_manager: Mutex<TargetManager>,
    pub daemon_cli: C::Fetcher,
}

pub type Reward = u64;

impl<C: Coin> Protocol for ProtocolP2P<C> {
    type Request = Vec<u8>;
    type Response = Vec<u8>;
    type Config = (ConfigP2P<C::BlockT>, Box<Path>, u16); // data dir, listening port
    type ClientContext = Peer;
    type ProcessingContext = ();

    fn new(conf: Self::Config) -> Self {
        let (config_p2p, data_dir, port) = conf;

        let daemon_cli = C::Fetcher::new(config_p2p.rpc_url.as_ref());
        let genesis_share =
            BlockManager::decode_share(config_p2p.consensus.genesis_share.clone(), &HashMap::new())
                .unwrap();

        Self {
            pplns_window: Mutex::new(WindowPPLNS::new(genesis_share.clone())),
            hello_message: Messages::Hello(Hello::new(port, &config_p2p.consensus)),
            target_manager: Mutex::new(TargetManager::new::<C>(
                &genesis_share.block,
                config_p2p.consensus.block_time,
                config_p2p.consensus.diff_adjust_blocks,
            )),
            block_manager: BlockManager::new(genesis_share, data_dir.clone()),
            peers: Mutex::new(HashMap::new()),
            conf: config_p2p,
            peer_manager: PeerManager::new(data_dir.clone()),
            daemon_cli,
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

impl<C: Coin> ProtocolP2P<C> {
    pub fn get_new_pool_config(
        rpc_url: String,
        diff1: u64,
        block_time_ms: u64,
    ) -> ConfigP2P<C::BlockT> {
        let daemon_cli = C::Fetcher::new(rpc_url.as_ref());

        let rewards = [(
            C::Address::from_string(DEV_ADDRESS_BTC_STR)
                .unwrap()
                .to_script(),
            pplns::MAX_SCORE,
        )];

        let block = daemon_cli
            .fetch_blocktemplate(rewards.into_iter(), U256::ZERO)
            .unwrap()
            // .expect("Failed to get block")
            .block;

        ConfigP2P {
            peer_connections: 32,
            consensus: ConsensusConfigP2P {
                parent_pool_id: U256::ZERO,
                block_time: Duration::from_millis(block_time_ms),
                diff_adjust_blocks: 16,
                genesis_share: block,
                password: None,
                target_1: difficulty::get_target_from_diff_units(diff1, &C::DIFF1),
            },
            rpc_url,
        }
    }

    #[doc(hidden)]
    pub fn parse_request(req: &[u8]) -> Result<Messages<C::BlockT>, bincode::Error> {
        bincode::deserialize(&req)
    }

    fn process_message(
        &self,
        ctx: Arc<Mutex<Peer>>,
        message: Messages<C::BlockT>,
    ) -> Option<Messages<C::BlockT>> {
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

    fn handle_hello(&self, hello: Hello, ctx: Arc<Mutex<Peer>>) -> Option<Messages<C::BlockT>> {
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

    fn handle_ver_ack(&self, ctx: Arc<Mutex<Peer>>) -> Option<Messages<C::BlockT>> {
        // listening port is already known as it was used to connect...
        let mut lock = ctx.lock().unwrap();
        lock.authorized = Some(CURRENT_VERSION);
        self.peer_manager.save_peer(&*lock);
        None
    }

    fn handle_get_shares(&self) -> Option<Messages<C::BlockT>> {
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
        share: C::BlockT,
    ) -> Option<Messages<C::BlockT>> {
        let targetman = self.target_manager.lock().unwrap();

        match self.block_manager.process_share(
            share,
            &targetman,
            &self.pplns_window.lock().unwrap(),
        ) {
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

    pub fn send_message(message: &Messages<C::BlockT>, stream: &TcpStream) {
        respond(stream, Self::serialize_message(message).as_ref())
    }

    pub fn serialize_message(message: &Messages<C::BlockT>) -> Vec<u8> {
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
