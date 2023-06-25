use bitcoin::{consensus::Encodable, hashes::Hash};
use crypto_bigint::{Encoding, U256};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{
    p2p::networking::{bitcoin::COINB1_SIZE, block::Block},
    sickrpc::RpcReqBody,
};

use super::{header::BlockHeader, job_fetcher::BlockFetch};

pub trait Job<T, E>: Clone + std::fmt::Debug {
    fn get_broadcast_message(id: u32, fetch: &BlockFetch<T>) -> E;
}

#[derive(Debug, Clone)]
pub struct JobBtc<BlockT, MessageT> {
    pub id: u32,
    pub block: BlockT,
    pub target: U256,
    pub height: u32,
    pub reward: u64,
    // todo: bytes?
    pub broadcast_message: MessageT,
}

impl<T: Block, E> JobBtc<T, E>
where
    JobBtc<T, E>: Job<T, E>,
{
    pub fn new(id: u32, fetch: BlockFetch<T>) -> Self {
        let target = fetch.block.get_header().get_target();
        JobBtc {
            id,
            target,
            broadcast_message: Self::get_broadcast_message(id, &fetch),
            block: fetch.block,
            height: fetch.height,
            reward: fetch.reward,
        }
    }
}

impl Job<bitcoin::Block, RpcReqBody> for JobBtc<bitcoin::Block, RpcReqBody> {
    fn get_broadcast_message(id: u32, fetch: &BlockFetch<bitcoin::Block>) -> RpcReqBody {
        let header = fetch.block.get_header();

        let mut cb_bytes = Vec::new();
        let res: &Result<usize, std::io::Error> =
            &fetch.block.txdata[0].consensus_encode(&mut cb_bytes);

        (
            String::from("mining.notify"),
            json!([
                hex::encode(id.to_be_bytes()),
                header.get_prev().to_string().to_ascii_lowercase(),
                hex::encode(&cb_bytes[..COINB1_SIZE]),
                hex::encode(&cb_bytes[COINB1_SIZE..]),
                calc_merkle_steps(fetch.tx_hashes.clone()),
                hex::encode(header.version.to_consensus().to_be_bytes()),
                hex::encode(header.bits.to_consensus().to_be_bytes()),
                hex::encode(header.time.to_be_bytes()),
                "true"
            ]),
        )
    }
}

pub fn calc_merkle_steps(mut hashes: Vec<[u8; 32]>) -> Vec<[u8; 32]> {
    let mut hash_count = hashes.len();

    let mut res = Vec::with_capacity(hash_count);

    while hash_count > 1 {
        if hash_count & 1 > 0 {
            hashes.push(*hashes.last().unwrap());
            hash_count += 1;
        }

        res.push(hashes[1]);

        // we can skip the first one as we won't use it (it's not even
        // known)

        for i in (2..hash_count).step_by(2) {
            let to_hash = [hashes[i], hashes[i + 1]].concat();

            let result = sha256d(&to_hash);

            hashes[i / 2] = result;
        }

        hash_count = hash_count / 2;
    }
    res
}

pub fn sha256d(src: &[u8]) -> [u8; 32] {
    let src = Sha256::digest(src);
    Sha256::digest(src).into()
}

#[cfg(test)]
mod tests {
    use crypto_bigint::U256;

    use super::calc_merkle_steps;

    #[test]
    fn test_steps() {
        let hashes = Vec::from([
            U256::from_be_hex("c1d1ae759572c9792338b12aaaa12548e136b4c5aeefb145bba8953925caf3e5"),
            U256::from_be_hex("359ed65c6377fc8a425d29acdd2b5e567b0bdaee0e003c7ed5df588a3346b3a0"),
            U256::from_be_hex("96b608b4b89aea059473c1eeef85a27b087cc660b166952bca7553e93ebbd664"),
            U256::from_be_hex("9dbc3445b2f5917976c7aa2aa54b7def2548e4dd8cfee86495ff5226e36d7c71"),
        ]);

        let steps = calc_merkle_steps(hashes);
        assert_eq!(
            steps,
            Vec::from([
                U256::from_le_hex(
                    "a0b346338a58dfd57e3c000eeeda0b7b565e2bddac295d428afc77635cd69e35"
                ),
                U256::from_le_hex(
                    "5fa3a8bfaf3cc7e9d15e1c63ff07f8837b882e3b09e915b507a6a9a31246a22c"
                )
            ])
        );
    }
}

// #[derive(Serialize_tuple)]
// struct JobParamsBtc {
//     id: u32,
//     prevhash: U256,

// }

// TODO: make job copy sturct for only the header
