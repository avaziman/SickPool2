use log::info;
use primitive_types::U256;

use crate::p2p::networking::protocol::Address;

use super::{job::Job, job_btc::BlockHeader};

pub enum ShareResult {
    Valid(U256),
    Block(U256),
    Stale(),
    Invalid(),
}

#[inline]
pub fn process_share<T: BlockHeader>(
    job: Option<&mut Job<T>>,
    params: T::SubmitParams,
    cli_diff: U256,
) -> ShareResult {
    match job {
        Some(job) => {
            job.header.update_fields(&params);

            let hash = job.header.get_hash();
            info!("Hash {}", hash);

            if hash >= job.target {
                ShareResult::Block(hash)
            } else if hash >= cli_diff {
                ShareResult::Valid(hash)
            } else {
                ShareResult::Invalid()
            }
        }
        None => ShareResult::Stale(),
    }
}