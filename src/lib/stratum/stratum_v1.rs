use bitcoin::hashes::Hash;
use bitcoincore_rpc::bitcoin::{self};

use crypto_bigint::{Encoding, U256};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use serde_hex::*;
use serde_tuple::Deserialize_tuple;

use crate::coins::coin::Coin;
use crate::p2p::networking::protocol::SubmittingContext;
use crate::p2p::networking::share::CoinbaseEncodedP2P;
use crate::{
    address::Address,
    coins::bitcoin::{Btc, MyBtcAddr},
};
use slab::Slab;
use std::fmt;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex, RwLock,
    },
    time::Instant,
};

use serde_json::{json, Value};

use crate::{
    p2p::networking::{
        block::Block, difficulty::get_target_from_diff_units, hard_config::PPLNS_SHARE_UNITS,
        protocol::ProtocolP2P, stratum_handler::CompleteStratumHandler,
    },
    protocol::{JsonRpcProtocol, Protocol},
    server::Notifier,
    sickrpc::RpcReqBody,
};

use super::{
    client::StratumClient,
    common::{process_share, ShareResult},
    config::StratumConfig,
    handler::StratumHandler,
    header::BlockHeader,
    job::JobBtc,
    job_fetcher::BlockFetcher,
    job_manager::JobManager,
    protocol::StratumProtocol,
};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum StratumRequestsBtc {
    #[serde(rename="mining.submit")]
    Submit(SubmitReqParams),
    #[serde(rename="mining.subscribe")]
    Subscribe,
    #[serde(rename="mining.authorize")]
    Authorize(AuthorizeReqParams),
}

#[derive(Serialize, Deserialize_tuple, PartialEq, Debug)]
pub struct AuthorizeReqParams {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize_tuple, PartialEq, Debug)]
pub struct SubmitReqParams {
    pub worker_name: String,
    #[serde(with = "SerHex::<Strict>")]
    pub job_id: u32,
    #[serde(with = "SerHex::<Strict>")]
    pub extranonce2: u32,
    #[serde(with = "SerHex::<Strict>")]
    pub time: u32,
    #[serde(with = "SerHex::<Strict>")]
    pub nonce: u32,
}

#[repr(u32)]
#[derive(Debug)]
pub enum StratumV1ErrorCodes {
    Other(String) = 20,
    JobNotFound = 21,
    DuplicateShare = 22,
    LowDifficultyShare = 23,
    UnauthorizedWorker = 24,
    NotSubscribed = 25,
}

pub trait Discriminant {
    fn discriminant(&self) -> u32;
}

impl Discriminant for StratumV1ErrorCodes {
    fn discriminant(&self) -> u32 {
        unsafe { *(self as *const Self as *const u32) }
    }
}

impl fmt::Display for StratumV1ErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StratumV1ErrorCodes::Other(reason) => write!(f, "{}", reason),
            StratumV1ErrorCodes::JobNotFound => write!(f, "Job not found"),
            StratumV1ErrorCodes::DuplicateShare => write!(f, "Duplicate share"),
            StratumV1ErrorCodes::LowDifficultyShare => write!(f, "Low difficulty share"),
            StratumV1ErrorCodes::UnauthorizedWorker => write!(f, "Unauthorized worker"),
            StratumV1ErrorCodes::NotSubscribed => write!(f, "Client not subscribed"),
        }
    }
}
// original slush bitcoin stratum protocol
pub struct StratumV1 {
    job_manager: RwLock<JobManager<JobBtc<bitcoin::Block, RpcReqBody>>>,
    client_count: AtomicU32,
    config: StratumConfig,
    pub handler: CompleteStratumHandler<Btc>,
    pub subscribed_clients: Mutex<Slab<Notifier>>,
    pub daemon_cli: <Btc as Coin>::Fetcher,
}

impl StratumV1 {
    pub fn process_stratum_request(
        &self,
        req: StratumRequestsBtc,
        ctx: Arc<Mutex<StratumClient>>,
        ptx: &mut StratumProcessingContext<<Btc as Coin>::BlockT, RpcReqBody>,
    ) -> Result<(Value, Vec<RpcReqBody>), StratumV1ErrorCodes> {
        let now = Instant::now();
        info!("Received stratum request: {:?}", req);

        let res = match req {
            StratumRequestsBtc::Submit(req) => self.process_submit(req, ctx, ptx),
            StratumRequestsBtc::Subscribe => {
                let mut lock = ctx.lock().unwrap();
                let key = self
                    .subscribed_clients
                    .lock()
                    .unwrap()
                    .insert(lock.notifier.clone());

                lock.subscription_key = Some(key);
                Ok((
                    json!([
                        [
                            ["mining.set_difficulty", Value::Null],
                            ["mining.notify", Value::Null]
                        ],
                        hex::encode(lock.extra_nonce.to_be_bytes()),
                        std::mem::size_of_val(&lock.extra_nonce),
                        // extranonce 2
                    ]),
                    Vec::new(),
                ))
            }
            StratumRequestsBtc::Authorize(params) => {
                // TODO: get address
                let _pk = match MyBtcAddr::from_string(&params.username) {
                    Ok(k) => k,
                    Err(_) => {
                        return Err(StratumV1ErrorCodes::Other(String::from(
                            "Invalid address provided",
                        )));
                    }
                };
                ctx.lock()
                    .unwrap()
                    .authorized_workers
                    .insert(params.username.clone(), params.username);

                let jobs = self.job_manager.read().unwrap();
                let job = jobs.last_job();

                let diff = self.config.default_diff_units;
                let notifs = Vec::from([
                    (
                        "mining.set_difficulty".into(),
                        json!([diff as f64 / PPLNS_SHARE_UNITS as f64]),
                    ),
                    job.broadcast_message.clone(),
                ]);
                ctx.lock().unwrap().target = get_target_from_diff_units(diff, &Btc::DIFF1);

                Ok((Value::Bool(true), notifs))
            }
        };

        let elapsed = now.elapsed().as_micros();
        info!("Processed stratum response in {}us: {:?}", elapsed, &res);
        res
    }

    fn process_submit(
        &self,
        params: SubmitReqParams,
        ctx: Arc<Mutex<StratumClient>>,
        ptx: &mut StratumProcessingContext<<Btc as Coin>::BlockT, RpcReqBody>,
    ) -> Result<(Value, Vec<RpcReqBody>), StratumV1ErrorCodes> {
        if !ptx
            .jobs
            .contains_key(&(self.job_manager.read().unwrap().get_job_count() - 1))
        {
            ptx.jobs = self.job_manager.read().unwrap().get_jobs()
        }

        let mut job = ptx.jobs.get_mut(&params.job_id);
        let mut lock = ctx.lock().unwrap();
        let address = match lock.authorized_workers.get(&params.worker_name) {
            Some(s) => s.clone(),
            None => return Err(StratumV1ErrorCodes::UnauthorizedWorker),
        };

        let res = process_share(&mut job, (params, lock.extra_nonce), &mut *lock);
        std::mem::drop(lock);

        match res {
            ShareResult::Block(diff) => {
                info!("Found block! {}", diff);
                let job = job.unwrap();
                if let Err(e) = self.daemon_cli.submit_block(&job.block) {
                    error!("Failed to submit block: {}", e);
                }

                self.handler.on_valid_share(
                    ctx.clone(),
                    &MyBtcAddr::from_string(&address).unwrap(),
                    &job.block,
                    diff,
                )
            }
            ShareResult::Valid(diff) => self.handler.on_valid_share(
                ctx.clone(),
                &MyBtcAddr::from_string(&address).unwrap(),
                &job.unwrap().block,
                diff,
            ),
            _ => {}
        };

        let res: Result<Value, StratumV1ErrorCodes> = res.into();
        match res {
            Ok(k) => Ok((k, Vec::new())),
            Err(e) => Err(e),
        }
    }

    pub fn parse_stratum_req(
        method: String,
        params: Value,
    ) -> Result<StratumRequestsBtc, serde_json::Error> {
        let obj = Value::Object(serde_json::Map::from_iter([(method, params)]));

        serde_json::from_value(obj)
    }
}

impl StratumProtocol for StratumV1 {
    type Coin = Btc;

    fn fetch_new_job(&self) {
        let mut lock = self.job_manager.write().unwrap();
        let res = lock.get_new_job(
            &self.daemon_cli,
            self.handler
                .p2p
                .pplns_window
                .lock()
                .unwrap()
                .address_scores
                .iter()
                .map(|(addr, score)| (addr.to_script(), *score)),
            {
                let tip = self.handler.p2p.block_manager.p2p_tip();
                CoinbaseEncodedP2P {
                    prev_hash: tip.hash,
                    height: tip.inner.encoded.height + 1,
                    round_num: self.handler.p2p.block_manager.round_num(),
                }
            },
        );

        if let Ok(job) = res {
            if let Some(job) = job {
                let lock = self.subscribed_clients.lock().unwrap();
                info!("New job! broadcasting to {} clients", lock.len(),);

                for (_token, notifier) in &*lock {
                    JsonRpcProtocol::<Self>::notify(job.broadcast_message.clone(), notifier);
                }
                // the received block is the one in the last job with the found params
                self.handler.on_new_block(
                    job.height,
                    &U256::from_le_bytes(
                        job.block
                            .header
                            .prev_blockhash
                            .as_raw_hash()
                            .to_byte_array(),
                    ),
                );
            }
        }
    }
}

impl<T: StratumProtocol, E> StratumProtocol for JsonRpcProtocol<T>
where
    E: std::fmt::Display + Discriminant,
    T: Protocol<Request = RpcReqBody, Response = Result<(Value, Vec<RpcReqBody>), E>>,
{
    type Coin = T::Coin;

    fn fetch_new_job(&self) {
        self.up.fetch_new_job()
    }
}

impl Into<Result<Value, StratumV1ErrorCodes>> for ShareResult {
    fn into(self) -> Result<Value, StratumV1ErrorCodes> {
        match self {
            ShareResult::Valid(_) | ShareResult::Block(_) => Ok(Value::Bool(true)),
            ShareResult::Stale() => Err(StratumV1ErrorCodes::JobNotFound),
            ShareResult::Invalid() => Err(StratumV1ErrorCodes::LowDifficultyShare),
            ShareResult::Duplicate() => Err(StratumV1ErrorCodes::DuplicateShare),
        }
    }
}

pub struct StratumProcessingContext<T, E> {
    pub jobs: HashMap<u32, JobBtc<T, E>>,
}

impl<T, E> Default for StratumProcessingContext<T, E>
// where
// T: BlockFetcher<BlockT = bitcoin::Block>,
{
    fn default() -> Self {
        StratumProcessingContext {
            jobs: HashMap::new(),
        }
    }
}

// any client that can generate the compatible header can be suited to this stratum protocol
impl Protocol for StratumV1 {
    // method, params
    type Request = RpcReqBody;
    type Response = Result<(Value, Vec<RpcReqBody>), StratumV1ErrorCodes>;
    type Config = (StratumConfig, Arc<ProtocolP2P<Btc>>);
    type ClientContext = StratumClient;
    type ProcessingContext = StratumProcessingContext<<Btc as Coin>::BlockT, RpcReqBody>;

    fn new(conf: Self::Config) -> Self {
        // let p = .clone();
        let (stratum_conf, p2p) = conf;
        let daemon_cli = <<Btc as Coin>::Fetcher as BlockFetcher<bitcoin::Block>>::new(
            stratum_conf.rpc_url.as_ref(),
        )
        .unwrap();

        StratumV1 {
            job_manager: RwLock::new(JobManager::new(&daemon_cli)),
            client_count: AtomicU32::new(1),
            subscribed_clients: Mutex::new(Slab::new()),
            daemon_cli,
            handler: CompleteStratumHandler { p2p },
            config: stratum_conf,
        }
    }

    fn process_request(
        &self,
        req: Self::Request,
        ctx: Arc<Mutex<Self::ClientContext>>,
        ptx: &mut Self::ProcessingContext,
    ) -> Self::Response {
        match Self::parse_stratum_req(req.0, req.1) {
            Ok(stratum_req) => self.process_stratum_request(stratum_req, ctx, ptx),
            Err(e) => {
                warn!("Failed to parse stratum request: {}", e);
                return Err(StratumV1ErrorCodes::Other(format!(
                    "Failed to parse stratum request: {}",
                    e
                )));
            }
        }
    }

    fn create_client(&self, addr: SocketAddr, notifier: Notifier) -> Option<Self::ClientContext> {
        let id = self.client_count.load(Ordering::Relaxed);
        self.client_count.store(id + 1, Ordering::Relaxed);
        Some(StratumClient::new(notifier, id, addr))
    }

    fn delete_client(&self, ctx: Arc<Mutex<Self::ClientContext>>) {
        let mut lock = ctx.lock().unwrap();
        if let Some(subkey) = lock.subscription_key.take() {
            self.subscribed_clients.lock().unwrap().remove(subkey);
        }

        // info!("Deleted client with token: {}", _token.0);
    }

    fn create_ptx(&self) -> Self::ProcessingContext {
        Self::ProcessingContext {
            jobs: self.job_manager.read().unwrap().get_jobs(),
        }
    }
}

// demo \n
/*
{"id": 1, "method": "mining.subscribe", "params": []}
{"params": ["slush.miner1", "password"], "id": 2, "method": "mining.authorize"}
{"params": ["slush.miner1", "00000000", "00000001", "504e86ed", "b2957c02"], "id": 4, "method": "mining.submit"}
 */
