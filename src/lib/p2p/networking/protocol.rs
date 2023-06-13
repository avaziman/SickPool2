use std::{
    collections::{HashMap, VecDeque},
    fs,
    net::IpAddr,
    sync::{Arc, Mutex},
};

use bitcoincore_rpc::bitcoin::secp256k1::Message;
use io_arc::IoArc;
use log::{info, warn};
use mio::net::TcpStream;
use primitive_types::U256;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::net::SocketAddr;

use crate::{protocol::Protocol, stratum::job_btc::BlockHeader};

use super::{
    discovery::discover_peers,
    hard_config::{self, OLDEST_COMPATIBLE_VERSION},
    peer::Peer,
};
use bincode::{self};

pub struct ProtocolP2P<HeaderT: BlockHeader + DeserializeOwned + Serialize + Clone> {
    state: State<HeaderT>,
    conf: ConfigP2P,
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

pub struct ConfigP2P {
    pub peer_connections: u32,
}

impl<HeaderT: BlockHeader + DeserializeOwned + Serialize + Clone> Protocol
    for ProtocolP2P<HeaderT>
{
    type Request = String;
    type Response = String;
    type Config = ConfigP2P;
    type ClientContext = Peer;
    type ProcessingContext = ();

    fn new(conf: Self::Config) -> Self {
        Self {
            state: State::new(),
            conf,
        }
    }

    fn peers_to_connect(&self) -> Vec<std::net::SocketAddr> {
        let peers: Vec<std::net::SocketAddr> = fs::read_to_string("peers.bin")
            .unwrap_or(String::new())
            .lines()
            .map(|s| s.parse().expect("Invalid peers"))
            .collect();

        // while (peers.len() as u32) < conf.peer_connections {
        //     info!("Discovering peers...");
        // }
        peers
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

    fn create_client(&self, address: SocketAddr, stream: IoArc<TcpStream>) -> Self::ClientContext {
        Self::ClientContext {
            address,
            successfully_connected: false,
        }
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
                    ctx.lock().unwrap().successfully_connected = true;
                    Some(Messages::VerAck)
                } else {
                    Some(Messages::Reject)
                }
            }
            Messages::VerAck => {
                ctx.lock().unwrap().successfully_connected = true;
                None
            }
            Messages::GetShares => Some(Messages::Shares(self.state.window_shares.get_shares())),
            Messages::Shares(shares) => todo!(),
            Messages::ShareSubmit(_) => todo!(),
            Messages::Reject => todo!(),
        }
    }
}
