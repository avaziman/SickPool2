use bitcoincore_rpc::bitcoin::{self, Address, PublicKey};
use hex::encode;
use io_arc::IoArc;
use itertools::Itertools;
use log::{info, warn};
use mio::{net::TcpStream, Token};
use slab::Slab;
use std::{
    collections::HashMap,
    net::SocketAddr,
    str::FromStr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, RwLock, MutexGuard,
    },
    time::Instant,
};

use serde_json::{json, Value};

use crate::{
    p2p::networking::{
        pplns::MyBtcAddr, protocol::ProtocolP2P, stratum_handler::CompleteStrartumHandler,
    },
    protocol::Protocol,
    server::{respond, Notifier},
    sickrpc::{RpcReqBody, RpcRequest},
};

use super::{
    client::StratumClient,
    common::{process_share, ShareResult},
    config::StratumConfig,
    handler::StratumHandler,
    job::Job,
    job_fetcher::BlockFetcher,
    job_manager::JobManager,
    protocol::{StratumRequestsBtc, StratumV1ErrorCodes, SubmitReqParams},
};

// original slush bitcoin stratum protocol
pub struct StratumV1<T: BlockFetcher> {
    job_manager: RwLock<JobManager<T>>,
    client_count: AtomicUsize,
    config: StratumConfig,
    pub handler: CompleteStrartumHandler<T::BlockT>,
    pub subscribed_clients: Mutex<Slab<Notifier>>,
    pub daemon_cli: T,
}

impl<T> StratumV1<T>
where
    T: BlockFetcher<BlockT = bitcoin::Block>,
{
    pub fn process_stratum_request(
        &self,
        req: StratumRequestsBtc,
        ctx: Arc<Mutex<StratumClient>>,
        ptx: &mut StratumProcessingContext<T>,
    ) -> Result<Value, StratumV1ErrorCodes> {
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
                Ok(json!([
                    [
                        ["mining.set_difficulty", Value::Null],
                        ["mining.notify", Value::Null]
                    ],
                    hex::encode(lock.extra_nonce.to_be_bytes()),
                    8,
                ]))
            }
            StratumRequestsBtc::Authorize(params) => {
                // TODO: get address
                let pk = match Address::from_str(&params.username) {
                    Ok(k) => {
                        // let atype = k.assume_checked().address_type();
                        // info!("Type: {:?}", atype);
                        k.require_network(bitcoin::Network::Bitcoin).unwrap()
                    }
                    Err(_) => {
                        return Err(StratumV1ErrorCodes::Unknown(String::from(
                            "Invalid address provided",
                        )));
                    }
                };
                ctx.lock()
                    .unwrap()
                    .authorized_workers
                    .insert(params.username, MyBtcAddr(pk));

                Ok(Value::Bool(true))
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
        ptx: &mut StratumProcessingContext<T>,
    ) -> Result<Value, StratumV1ErrorCodes> {
        if !ptx
            .jobs
            .contains_key(&self.job_manager.read().unwrap().get_job_count())
        {
            ptx.jobs = self.job_manager.read().unwrap().get_jobs()
        }

        let mut job = ptx.jobs.get_mut(&params.job_id);
        let mut lock = ctx.lock().unwrap();
        let _difficulty = lock.difficulty;
        let address = match lock.authorized_workers.get(&params.worker_name) {
            Some(s) => s.clone(),
            None => return Err(StratumV1ErrorCodes::UnauthorizedWorker),
        };

        let res = process_share(&mut job, params, &mut *lock);

        match res {
            ShareResult::Valid(diff) | ShareResult::Block(diff) => {
                self.handler
                    .on_valid_share(address, &job.unwrap().block, diff)
            }
            _ => {}
        };

        res.into()
    }

    pub fn parse_stratum_req(
        method: String,
        params: Value,
    ) -> Result<StratumRequestsBtc, serde_json::Error> {
        match method.as_ref() {
            "mining.submit" => Ok(StratumRequestsBtc::Submit(serde_json::from_value(params)?)),
            "mining.authorize" => Ok(StratumRequestsBtc::Authorize(serde_json::from_value(
                params,
            )?)),
            "mining.subscribe" => Ok(StratumRequestsBtc::Subscribe),
            unknown => Err(serde::de::Error::custom(format!(
                "Unknown method: {}",
                unknown
            ))),
        }
    }

    pub fn fetch_new_job(&self, header_fetcher: &T) -> Option<Job<T::BlockT>> {
        let mut lock = self.job_manager.write().unwrap();
        let res = lock.get_new_job(header_fetcher);

        if let Ok(job) = res {
            if let Some(job) = job {
                // let lock = self.subscribed_clients.lock().unwrap();
                // info!(
                //     "New job! broadcasting to {} clients: {:?}",
                //     lock.len(),
                //     lock
                // );

                return Some(job.clone());

                // for (_token, notifier) in &*lock {
                //     // notifier.notify(job.get_broadcast_message());
                //     // respond(stream.as_ref(), .as_bytes());
                // }
                // self.handler.on_new_block(job.height, &job.block);
            }
        }
        None
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

pub struct StratumProcessingContext<RpcClient: BlockFetcher> {
    pub jobs: HashMap<u32, Job<RpcClient::BlockT>>,
}

impl<T> Default for StratumProcessingContext<T>
where
    T: BlockFetcher<BlockT = bitcoin::Block>,
{
    fn default() -> Self {
        StratumProcessingContext {
            jobs: HashMap::new(),
        }
    }
}

// any client that can generate the compatible header can be suited to this stratum protocol
impl<T> Protocol for StratumV1<T>
where
    T: BlockFetcher<BlockT = bitcoin::Block>,
{
    // method, params
    type Request = RpcReqBody;
    type Response = Result<Value, StratumV1ErrorCodes>;
    type Config = (StratumConfig, Arc<ProtocolP2P<T::BlockT>>);
    type ClientContext = StratumClient;
    type ProcessingContext = StratumProcessingContext<T>;
    type Notification = Self::Request;

    fn new(conf: Self::Config) -> Self {
        let daemon_cli = T::new(conf.0.rpc_url.as_ref());

        StratumV1 {
            job_manager: RwLock::new(JobManager::new(&daemon_cli)),
            client_count: AtomicUsize::new(0),
            subscribed_clients: Mutex::new(Slab::new()),
            daemon_cli,
            handler: CompleteStrartumHandler { p2p: conf.1 },
            config: conf.0,
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
                return Err(StratumV1ErrorCodes::Unknown(format!(
                    "Failed to parse stratum request: {}",
                    e
                )));
            }
        }
    }

    fn create_client(&self, _addr: SocketAddr, notifier: Notifier) -> Option<Self::ClientContext> {
        let id = self.client_count.load(Ordering::Relaxed);
        self.client_count.store(id + 1, Ordering::Relaxed);
        Some(StratumClient::new(notifier, id))
    }

    fn delete_client(&self, ctx: Arc<Mutex<Self::ClientContext>>) {
        let lock = ctx.lock().unwrap();
        if let Some(subkey) = lock.subscription_key {
            self.subscribed_clients.lock().unwrap().remove(subkey);
        }

        // info!("Deleted client with token: {}", _token.0);
    }

    fn create_ptx(&self) -> Self::ProcessingContext {
        Self::ProcessingContext {
            jobs: self.job_manager.read().unwrap().get_jobs(),
        }
    }

    fn poll_notifications(&self) -> (MutexGuard<Slab<Notifier>>, Self::Request) {
        std::thread::sleep(self.config.job_poll_interval);
        // info!("Polling job...");

        loop {
            if let Some(job) = self.fetch_new_job(&self.daemon_cli) {
                let msg = job.get_broadcast_message();
                let lock = self.subscribed_clients.lock().unwrap();
                return (lock, msg);
            }
        }
    }
}

// demo \n
/*
{"id": 1, "method": "mining.subscribe", "params": []}
{"params": ["slush.miner1", "password"], "id": 2, "method": "mining.authorize"}
{"params": ["slush.miner1", "00000000", "00000001", "504e86ed", "b2957c02"], "id": 4, "method": "mining.submit"}
 */
