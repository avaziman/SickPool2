use crypto_bigint::U256;

use crate::{p2p::networking::block::Block};

use super::{
    client::StratumClient,
    header::BlockHeader,
    job::{Job, JobBtc},
};

pub enum ShareResult {
    Valid(U256),
    Block(U256),
    Stale(),
    Invalid(),
    Duplicate(),
}

#[inline]
pub fn process_share<B: Block, E>(
    job: &mut Option<&mut JobBtc<B, E>>,
    params: <JobBtc<B, E> as Job<B, E>>::SubmitParams,
    client: &mut StratumClient,
) -> ShareResult
where
    JobBtc<B, E>: Job<B, E>,
{
    match job {
        Some(job) => {
            job.update_fields(&params); 
            let hash = job.block.get_header().get_hash();

            if client.submitted_shares.did_contain(&hash) {
                return ShareResult::Duplicate();
            }

            // log::info!("Hash {:x}", hash);

            if hash <= job.target {
                ShareResult::Block(hash)
            } else if hash <= client.target {
                ShareResult::Valid(hash)
            } else {
                ShareResult::Invalid()
            }
        }
        None => ShareResult::Stale(),
    }
}
