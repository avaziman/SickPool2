use std::collections::HashMap;

use log::info;

use crate::{p2p::networking::{block::Block, protocol::Address}, stratum::job::Job};

use super::{header::BlockHeader, job::JobBtc, job_fetcher::BlockFetcher};

pub struct JobManager<JobT> {
    job_count: u32,
    jobs: HashMap<u32, JobT>,
}

// job manager is responsible for generating and updating jobs, the only one that can mutate jobs
impl<BlockT: Block, E> JobManager<JobBtc<BlockT, E>>
where
    JobBtc<BlockT, E>: Job<BlockT, E>,
{
    pub fn new<Fetcher: BlockFetcher<BlockT = BlockT>>(
        header_fetcher: &Fetcher,
    ) -> JobManager<JobBtc<BlockT, E>> {
        let mut jobs = HashMap::with_capacity(16);

        // this is an invalid job, no outputs TODO: ...
        match header_fetcher.fetch_block(&HashMap::new()) {
            Ok(res) => {
                let id = 0;
                let job = JobBtc::new(id, res);

                info!("First job: {:#?}", job);

                jobs.insert(id, job);
            }
            Err(e) => panic!("Failed to generate 1st job: {}", e),
        }

        JobManager { job_count: 1, jobs }
    }

    pub fn get_new_job<Fetcher: BlockFetcher<BlockT = BlockT>>(
        &mut self,
        header_fetcher: &Fetcher,
        vout: &HashMap<Address, u64>
    ) -> Result<Option<&JobBtc<BlockT, E>>, Fetcher::ErrorT> {
        let fetched = header_fetcher.fetch_block(vout)?;

        if fetched
            .block
            .get_header()
            .equal(&self.jobs[&(self.job_count - 1)].block.get_header())
        {
            return Ok(None);
        }

        let id = self.job_count;
        let job = JobBtc::new(id, fetched);

        self.job_count += 1;

        self.jobs.insert(id, job);

        Ok(Some(self.jobs.get(&id).unwrap()))
    }

    pub fn get_job_count(&self) -> u32 {
        self.job_count
    }

    pub fn get_jobs(&self) -> HashMap<u32, JobBtc<BlockT, E>> {
        self.jobs.clone()
    }
}
