use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, RwLock,
    },
    time::Instant,
};

use bitcoincore_rpc::bitcoin::{self};
use io_arc::IoArc;
use log::info;
use mio::{net::TcpStream, Token};
use primitive_types::U256;
use serde_json::Value;

use crate::{
    p2p::networking::{
        protocol::{Address, ProtocolP2P},
        stratum_handler::CompleteStrartumHandler,
    },
    protocol::Protocol,
    server::respond,
    sickrpc::RpcReqBody,
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
    pub handler: CompleteStrartumHandler<T::BlockT>,
    pub subscribed_clients: Mutex<HashMap<Token, IoArc<TcpStream>>>,
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
                let lock = ctx.lock().unwrap();
                self.subscribed_clients
                    .lock()
                    .unwrap()
                    .insert(lock.token, lock.stream.clone());
                Ok(Value::Bool(true))
            }
            StratumRequestsBtc::Authorize(params) => {
                // TODO: get address
                ctx.lock()
                    .unwrap()
                    .authorized_workers
                    .insert(params.username, Address::new());
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
        let difficulty = lock.difficulty;
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

    pub fn fetch_new_job(&self, header_fetcher: &T) {
        let mut lock = self.job_manager.write().unwrap();
        let res = lock.get_new_job(header_fetcher);

        if let Ok(job) = res {
            if let Some(job) = job {
                let lock = self.subscribed_clients.lock().unwrap();
                info!(
                    "New job! broadcasting to {} clients: {:?}",
                    lock.len(),
                    lock
                );

                for (token, stream) in &*lock {
                    respond(stream.clone(), "NEW JOB".as_bytes());
                }
                self.handler.on_new_block(job.height, &job.block);
            }
        }
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

    fn new(conf: Self::Config) -> Self {
        let daemon_cli = T::new(conf.0.rpc_url.as_ref());

        StratumV1 {
            job_manager: RwLock::new(JobManager::new(&daemon_cli)),
            client_count: AtomicUsize::new(0),
            subscribed_clients: Mutex::new(HashMap::new()),
            daemon_cli,
            handler: CompleteStrartumHandler { p2p: conf.1 },
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
                return Err(StratumV1ErrorCodes::Unknown(format!(
                    "Failed to parse request: {}",
                    e
                )));
            }
        }
    }

    fn create_client(
        &self,
        addr: SocketAddr,
        stream: IoArc<TcpStream>,
        token: mio::Token,
    ) -> Option<Self::ClientContext> {
        let id = self.client_count.load(Ordering::Relaxed);
        self.client_count.store(id + 1, Ordering::Relaxed);
        Some(StratumClient::new(stream, token, id))
    }

    fn delete_client(&self, addr: SocketAddr, ctx: Arc<Mutex<Self::ClientContext>>, token: Token) {
        let lock = ctx.lock().unwrap();
        self.subscribed_clients.lock().unwrap().remove(&lock.token);
    }
}

// demo \n
/*
{"id": 1, "method": "mining.subscribe", "params": []}
{"params": ["slush.miner1", "password"], "id": 2, "method": "mining.authorize"}
{"params": ["slush.miner1", "00000000", "00000001", "504e86ed", "b2957c02"], "id": 4, "method": "mining.submit"}
 */
