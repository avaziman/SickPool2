use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bitcoincore_rpc::bitcoin::{self, Target};
use log::info;
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
    pub job_manager: JobManager<T>,
    // job_manager: JobManager::new(RpcClient::new(config.rpc_url.as_ref())),
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
            StratumRequestsBtc::Authorize(_) => Ok(Value::Bool(true)),
        }
    }

    fn process_submit(
        &self,
        params: SubmitReqParams,
        ctx: Arc<Mutex<StratumClient>>,
        ptx: &mut StratumProcessingContext<T>,
    ) -> Result<Value, StratumV1ErrorCodes> {
        if !ptx.jobs.contains_key(&self.job_manager.get_job_count()) {
            ptx.jobs = self.job_manager.get_jobs()
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
            unknown => Err(serde::de::Error::custom(format!(
                "Unknown method: {}",
                unknown
            ))),
        }
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
        StratumV1 {
            job_manager: JobManager::new(&T::new(conf.rpc_url.as_ref())),
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
                return Err(StratumV1ErrorCodes::Unknown(String::from(format!(
                    "Failed to parse request: {}",
                    e
                ))));
            }
        }
    }
}
