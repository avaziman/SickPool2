use std::{
    collections::{HashMap},
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, RwLock,
    },
    thread,
};

use bitcoincore_rpc::bitcoin::{self};
use io_arc::IoArc;
use log::info;
use mio::{net::TcpStream, Token};
use primitive_types::U256;
use serde_json::Value;

use crate::{protocol::Protocol, sickrpc::RpcReqBody};

use super::{
    client::StratumClient,
    config::StratumConfig,
    job::Job,
    job_btc::BlockHeader,
    job_fetcher::HeaderFetcher,
    job_manager::JobManager,
    protocol::{StratumRequestsBtc, StratumV1ErrorCodes, SubmitReqParams},
};

// original slush bitcoin stratum protocol
pub struct StratumV1<T: HeaderFetcher> {
    job_manager: RwLock<JobManager<T>>,
    client_count: AtomicUsize,
    pub subscribed_clients: Mutex<HashMap<Token, IoArc<TcpStream>>>,
    pub daemon_cli: T,
}

impl<T> StratumV1<T>
where
    T: HeaderFetcher<HeaderT = bitcoin::block::Header>,
{
    pub fn process_stratum_request(
        &self,
        req: StratumRequestsBtc,
        ctx: Arc<Mutex<StratumClient>>,
        ptx: &mut StratumProcessingContext<T>,
    ) -> Result<Value, StratumV1ErrorCodes> {
        match req {
            StratumRequestsBtc::Submit(req) => self.process_submit(req, ctx, ptx),
            StratumRequestsBtc::Subscribe => {
                self.subscribed_clients
                    .lock()
                    .unwrap()
                    .insert(Token(0), ctx.lock().unwrap().stream.clone());
                Ok(Value::Bool(true))
            }
            StratumRequestsBtc::Authorize(_) => Ok(Value::Bool(true)),
        }
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

        let job = match ptx.jobs.get_mut(&params.job_id) {
            Some(job) => {
                job.header.update_fields(&params);
                job
            }
            None => return Err(StratumV1ErrorCodes::JobNotFound),
        };

        let hash = job.header.get_hash();
        let hash_target = U256::from(hash);
        info!("Hash {}", hash_target);

        if hash_target >= job.target {
            Ok(Value::Bool(true))
        } else if hash_target >= ctx.lock().unwrap().difficulty {
            Ok(Value::Bool(true))
        } else {
            Err(StratumV1ErrorCodes::LowDifficultyShare)
        }
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

    pub fn fetch_new_job(&self, header_fetcher: &T) -> bool {
        let mut lock = self.job_manager.write().unwrap();
        let res = lock.get_new_job(header_fetcher);

        if let Ok(r) = res {
            if let Some(r) = r {
                return true;
            }
        }
        false
    }
}

pub struct StratumProcessingContext<RpcClient: HeaderFetcher> {
    pub jobs: HashMap<u32, Job<RpcClient::HeaderT>>,
}

impl<T> Default for StratumProcessingContext<T>
where
    T: HeaderFetcher<HeaderT = bitcoin::block::Header>,
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
    T: HeaderFetcher<HeaderT = bitcoin::block::Header>,
{
    // method, params
    type Request = RpcReqBody;
    type Response = Result<Value, StratumV1ErrorCodes>;
    type Config = StratumConfig;
    type ClientContext = StratumClient;
    type ProcessingContext = StratumProcessingContext<T>;

    fn new(conf: Self::Config) -> Self {
        let daemon_cli = T::new(conf.rpc_url.as_ref());

        StratumV1 {
            job_manager: RwLock::new(JobManager::new(&daemon_cli)),
            client_count: AtomicUsize::new(0),
            subscribed_clients: Mutex::new(HashMap::new()),
            daemon_cli,
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

    fn create_client(&self, addr: SocketAddr, stream: IoArc<TcpStream>) -> Option<Self::ClientContext> {
        let id = self.client_count.load(Ordering::Relaxed);
        self.client_count.store(id + 1, Ordering::Relaxed);
        Some(StratumClient::new(stream, id))
    }
}

// TODO REMOVE FROM SUBSCRIBED CLIETNS