use crypto_bigint::U256;
use log::info;

use crate::p2p::networking::block::Block;

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
pub fn process_share<T: Block, E>(
    job: &mut Option<&mut JobBtc<T, E>>,
    params: <JobBtc<T, E> as Job<T, E>>::SubmitParams,
    client: &mut StratumClient,
) -> ShareResult
where
    JobBtc<T, E>: Job<T, E>,
{
    match job {
        Some(job) => {
            job.update_fields(&params);

            let hash = job.block.get_header().get_hash();

            let low = hash.as_words()[0];

            if client.submitted_shares.contains(&low) {
                return ShareResult::Duplicate();
            }

            client.submitted_shares.insert(low);

            info!("Hash {:x}", hash);
            info!("Target {:x}", client.target);

            /* if hash <= job.target {
                ShareResult::Block(hash)
            } else  */if hash <= client.target {
                ShareResult::Valid(hash)
            } else {
                ShareResult::Invalid()
            }
        }
        None => ShareResult::Stale(),
    }
}
