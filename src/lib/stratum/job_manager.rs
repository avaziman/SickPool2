use std::collections::HashMap;

use log::info;

use crate::p2p::networking::block::Block;

use super::{job::Job, header::BlockHeader, job_fetcher::BlockFetcher};

pub struct JobManager<Fetcher: BlockFetcher> {
    job_count: u32,
    jobs: HashMap<u32, Job<Fetcher::BlockT>>,
}

// job manager is responsible for generating and updating jobs, the only one that can mutate jobs
impl<Fetcher: BlockFetcher> JobManager<Fetcher> {
    pub fn new(header_fetcher: &Fetcher) -> JobManager<Fetcher> {
        let mut jobs = HashMap::with_capacity(16);

        match header_fetcher.fetch_block() {
            Ok((header, height)) => {
                let job = Job::new(0, header, height);

                info!("First job: {:#?}", job);

                jobs.insert(job.id, job);
            }
            Err(e) => panic!("Failed to generate 1st job: {}", e),
        }

        JobManager { job_count: 1, jobs }
    }

    pub fn get_new_job(
        &mut self,
        header_fetcher: &Fetcher,
    ) -> Result<Option<&Job<Fetcher::BlockT>>, Fetcher::ErrorT> {
        let (header, height) = header_fetcher.fetch_block()?;

        if header
            .get_header()
            .equal(&self.jobs[&(self.job_count - 1)].block.get_header())
        {
            return Ok(None);
        }

        let job = Job::new(self.job_count, header, height);

        self.job_count += 1;

        let id = job.id;
        self.jobs.insert(job.id, job);

        Ok(Some(self.jobs.get(&id).unwrap()))
    }

    pub fn get_job_count(&self) -> u32 {
        self.job_count
    }

    pub fn get_jobs(&self) -> HashMap<u32, Job<Fetcher::BlockT>> {
        self.jobs.clone()
    }
}
