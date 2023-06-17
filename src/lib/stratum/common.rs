use log::info;
use primitive_types::U256;

use crate::p2p::networking::protocol::Address;

use super::{job::Job, job_btc::BlockHeader, client::StratumClient};

pub enum ShareResult {
    Valid(U256),
    Block(U256),
    Stale(),
    Invalid(),
    Duplicate(),
}

#[inline]
pub fn process_share<T: BlockHeader>(
    job: Option<&mut Job<T>>,
    params: T::SubmitParams,
    client: &mut StratumClient,
) -> ShareResult {
    match job {
        Some(job) => {
            job.header.update_fields(&params);
            let share = job.header.clone();

            let hash = job.header.get_hash();
            let low = hash.low_u64();
            if client.submitted_shares.contains(&low) {
                return ShareResult::Duplicate();
            }

            client.submitted_shares.insert(low);

            info!("Hash {}", hash);

            if hash >= job.target {
                ShareResult::Block(hash)
            } else if hash >= client.difficulty {
                ShareResult::Valid(hash)
            } else {
                ShareResult::Invalid()
            }
        }
        None => ShareResult::Stale(),
    }
}