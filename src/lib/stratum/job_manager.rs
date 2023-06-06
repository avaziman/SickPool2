use std::collections::HashMap;

use log::info;

use super::{job::Job, job_btc::BlockHeader, job_fetcher::HeaderFetcher};

pub struct JobManager<RpcClient: HeaderFetcher> {
    job_count: u32,
    jobs: HashMap<u32, Job<RpcClient::HeaderT>>,
}

// job manager is responsible for generating and updating jobs, the only one that can mutate jobs
impl<RpcClient: HeaderFetcher> JobManager<RpcClient> {
    pub fn new(header_fetcher: &RpcClient) -> JobManager<RpcClient> {
        let mut jobs = HashMap::with_capacity(16);

        match header_fetcher.fetch_header() {
            Ok(header) => {
                let job = Job::new(0, header);

                info!("First job: {:#?}", job);

                jobs.insert(job.id, job);
            }
            Err(e) => panic!("Failed to generate 1st job: {}", e),
        }

        JobManager {
            job_count: jobs.len() as u32,
            jobs,
        }
    }

    pub fn get_new_job(&mut self, header_fetcher: &RpcClient) -> Result<&Job<RpcClient::HeaderT>, RpcClient::ErrorT> {
        let header = header_fetcher.fetch_header()?; 

        let job = Job::new(self.job_count, header);
        self.job_count += 1;
        // info!("New job: {:#?}", job);
        let id = job.id;
        self.jobs.insert(job.id, job);
        
        Ok(self.jobs.get(&id).unwrap())
    }

    pub fn update_job(
        &mut self,
        params: &<RpcClient::HeaderT as BlockHeader>::SubmitParams,
        job_id: u32,
    ) -> Option<&Job<RpcClient::HeaderT>> {
        let job = self.jobs.get_mut(&job_id);

        match job {
            Some(job) => {
                job.header.update_fields(params);
                Some(&*job)
            }
            None => None,
        }
    }
}
