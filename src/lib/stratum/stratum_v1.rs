use bitcoincore_rpc::bitcoin::{self, Target};
use log::info;
use serde_json::Value;

use crate::{protocol::Protocol, sickrpc::RpcReqBody};

use super::{
    config::StratumConfig,
    job_btc::BlockHeader,
    job_fetcher::HeaderFetcher,
    job_manager::JobManager,
    protocol::{StratumRequestsBtc, StratumV1ErrorCodes, SubmitReqParams}, client::StratumClient,
};

// original slush bitcoin stratum protocol
pub struct StratumV1<T: HeaderFetcher> {
    pub job_manager: JobManager<T>,
    // job_manager: JobManager::new(RpcClient::new(config.rpc_url.as_ref())),
}

// pub struct BaseHandler<T: HeaderFetcher> {
//     job_manager: JobManager<T>,
// }

// impl<T> StratumProtocolHandler for StratumV1<T>
// where
//     T: HeaderFetcher<HeaderT = bitcoin::block::Header>,
// {
//     fn new(job_manager: JobManager<Self::CompatibleClient>) -> Self {
//         StratumV1 { job_manager }
//     }

//     fn process_request(
//         &mut self,
//         request: Self::Request,
//     ) -> Self::Response
//     {
//         RpcResponse::new(request.id, self.process_stratum_request(request.stratum_request))
//     }
// }

impl<T> StratumV1<T>
where
    T: HeaderFetcher<HeaderT = bitcoin::block::Header>,
{
    pub fn process_stratum_request(
        &mut self,
        req: StratumRequestsBtc,
    ) -> Result<Value, StratumV1ErrorCodes> {
        match req {
            StratumRequestsBtc::Submit(req) => self.process_submit(req),
            StratumRequestsBtc::Authorize(_) => Ok(Value::Bool(true)),
        }
    }

    fn process_submit(&mut self, params: SubmitReqParams) -> Result<Value, StratumV1ErrorCodes> {
        let job = match self.job_manager.update_job(&params, params.job_id) {
            Some(job) => job,
            None => return Err(StratumV1ErrorCodes::JobNotFound),
        };

        let hash = job.header.get_hash();
        let hash_target = Target::from_le_bytes(hash);
        info!("Hash {}", hash_target.difficulty_float());

        if hash_target >= job.target {
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
    // type CompatibleHeader = T::HeaderT;
    // type CompatibleClient = T;
    fn new(conf: Self::Config) -> Self {
        StratumV1 {
            job_manager: JobManager::new(&T::new(conf.rpc_url.as_ref())),
        }
    }

    fn process_request(&mut self, req: Self::Request) -> Self::Response {
        let stratum_req: StratumRequestsBtc = match Self::parse_stratum_req(req.0, req.1) {
            Ok(k) => k,
            Err(e) => {
                return Err(StratumV1ErrorCodes::Unknown(String::from(
                    format!("Failed to parse request: {}", e)
                )));
            }
        };

        self.process_stratum_request(stratum_req)
    }
}
