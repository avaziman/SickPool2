use bitcoin::{
    consensus::Encodable, hash_types::TxMerkleNode, hashes::Hash,
};
use crypto_bigint::{ U256};

use itertools::Itertools;

use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{
    p2p::networking::{
        bitcoin::SCRIPTLESS_COINB1_SIZE,
    },
    sickrpc::RpcReqBody,
};

use super::{header::BlockHeader, job_fetcher::BlockFetch, protocol::SubmitReqParams};

pub trait Job<T, E>: Clone + std::fmt::Debug {
    type SubmitParams;
    fn update_fields(&mut self, params: &Self::SubmitParams);

    fn get_broadcast_message(id: u32, fetch: &BlockFetch<T>, merkle_steps: &Vec<[u8; 32]>) -> E;
}

#[derive(Debug, Clone)]
pub struct JobBtc<BlockT, MessageT> {
    pub id: u32,
    pub block: BlockT,
    pub target: U256,
    pub height: u32,
    pub reward: u64,
    pub merkle_steps: Vec<[u8; 32]>,
    // todo: bytes?
    pub broadcast_message: MessageT,
}

impl</* Address, */ E> JobBtc<bitcoin::Block, E>
where
    // bitcoin::Block: Block<Address>,
    JobBtc<bitcoin::Block, E>: Job<bitcoin::Block, E>,
{
    pub fn new(id: u32, fetch: BlockFetch<bitcoin::Block>) -> Self {
        let target = fetch.block.header.get_target();
        // TODO: avoid clone, its unnececsergfsdf
        let merkle_steps = calc_merkle_steps(fetch.tx_hashes.clone());
        JobBtc {
            id,
            target,
            broadcast_message: Self::get_broadcast_message(id, &fetch, &merkle_steps),
            block: fetch.block,
            height: fetch.height,
            reward: fetch.reward,
            merkle_steps,
        }
    }

    pub fn format_prev_hash(hash: &U256) -> String {
        let words = hash.to_words();
        let mut prev_hash_str = String::with_capacity(64);

        for word in words.into_iter() {
            let word1 = word as u32;
            let word2 = (word >> 32) as u32;
            prev_hash_str.push_str(&hex::encode(word1.to_be_bytes()));
            prev_hash_str.push_str(&hex::encode(word2.to_be_bytes()));
        }
        prev_hash_str
    }
}

impl Job<bitcoin::Block, RpcReqBody> for JobBtc<bitcoin::Block, RpcReqBody> {
    type SubmitParams = (SubmitReqParams, u32); //extra nonce 1

    fn get_broadcast_message(
        id: u32,
        fetch: &BlockFetch<bitcoin::Block>,
        merkle_steps: &Vec<[u8; 32]>,
    ) -> RpcReqBody {
        let header = fetch.block.header;

        let mut cb_bytes = Vec::new();
        let _res: &Result<usize, std::io::Error> =
            &fetch.block.txdata[0].consensus_encode(&mut cb_bytes);

        let script_size = fetch.block.txdata[0].input[0].script_sig.len();
        let prev_hash_str = Self::format_prev_hash(&header.get_prev());
        let coinb1_size = SCRIPTLESS_COINB1_SIZE + script_size;
        (
            String::from("mining.notify"),
            json!([
                hex::encode(id.to_be_bytes()),
                prev_hash_str,
                hex::encode(&cb_bytes[..coinb1_size - 7]),
                hex::encode(&cb_bytes[(coinb1_size + 1)..]),
                merkle_steps
                    .iter()
                    .map(|h| { hex::encode(h) })
                    .collect_vec(),
                hex::encode(header.version.to_consensus().to_be_bytes()),
                hex::encode(header.bits.to_consensus().to_be_bytes()),
                hex::encode(header.time.to_be_bytes()),
                "true"
            ]),
        )
    }

    fn update_fields(&mut self, params: &(SubmitReqParams, u32)) {
        let (params, extra_nonce1) = params;
        self.block.header.nonce = params.nonce;
        self.block.header.time = params.time;

        let extra_nonce = ((*extra_nonce1).to_be() as u64) + ((params.extranonce2 as u64) << 32);
        // let extra_nonce = 1u64;

        // insert second nonce
        let len = self.block.txdata[0].input[0].script_sig.len();
        self.block.txdata[0].input[0].script_sig.as_mut_bytes()
            [len-8..len]
            .copy_from_slice(&extra_nonce.to_le_bytes());

        // recalculate cb hash and merkle root
        let cb_txid = self.block.txdata[0].txid().to_byte_array();

        let merkle_root = build_merkle_root_from_steps(cb_txid, &self.merkle_steps);

        // be
        self.block.header.merkle_root = TxMerkleNode::from_byte_array(merkle_root);
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

pub fn build_merkle_root_from_steps(cb: [u8; 32], steps: &Vec<[u8; 32]>) -> [u8; 32] {
    let mut root = cb;

    for step in steps {
        let to_hash = [root, step.clone()].concat();

        root = sha256d(&to_hash)
    }
    root
}

pub fn sha256d(src: &[u8]) -> [u8; 32] {
    let src = Sha256::digest(src);
    Sha256::digest(src).into()
}

#[cfg(test)]
mod tests {

    use crypto_bigint::U256;

    use crate::{sickrpc::RpcReqBody, stratum::job::build_merkle_root_from_steps};

    use super::{calc_merkle_steps, JobBtc};

    fn hex_to_arr<const N: usize>(s: &str) -> [u8; N] {
        hex::decode(s).unwrap().try_into().unwrap()
    }

    #[test]
    fn test_steps() {
        let hashes = Vec::from([
            hex_to_arr("E5F3CA253995A8BB45B1EFAEC5B436E14825A1AA2AB1382379C9729575AED1C1"),
            hex_to_arr("A0B346338A58DFD57E3C000EEEDA0B7B565E2BDDAC295D428AFC77635CD69E35"),
            hex_to_arr("64D6BB3EE95375CA2B9566B160C67C087BA285EFEEC1739405EA9AB8B408B696"),
            hex_to_arr("717C6DE32652FF9564E8FE8CDDE44825EF7D4BA52AAAC7767991F5B24534BC9D"),
        ]);

        let steps = calc_merkle_steps(hashes);
        assert_eq!(
            steps,
            Vec::from([
                hex_to_arr("a0b346338a58dfd57e3c000eeeda0b7b565e2bddac295d428afc77635cd69e35"),
                hex_to_arr("5fa3a8bfaf3cc7e9d15e1c63ff07f8837b882e3b09e915b507a6a9a31246a22c")
            ])
        );

        let root = build_merkle_root_from_steps(
            hex_to_arr("e5f3ca253995a8bb45b1efaec5b436e14825a1aa2ab1382379c9729575aed1c1"),
            &steps,
        );
        assert_eq!(
            root,
            hex_to_arr("f01b9e318508b61c335bd856efb27ad7826fc8363878e95e45a9c6f361fbfd03")
        );
    }

    #[test]
    fn test_steps2() {
        let hashes = Vec::from([
            hex_to_arr("B735924B863A7C07C78A116563B70C37D43341919CA291E5E030A71E0858A6BA"),
            hex_to_arr("B37631D59688DD9712DA1057654EF9685D1B22DB75A2BE187FF6FAD691EE9D7A"),
        ]);

        let steps = calc_merkle_steps(hashes);
        assert_eq!(
            steps,
            Vec::from([hex_to_arr(
                "b37631d59688dd9712da1057654ef9685d1b22db75a2be187ff6fad691ee9d7a"
            ),])
        );

        let root = build_merkle_root_from_steps(
            hex_to_arr("b735924b863a7c07c78a116563b70c37d43341919ca291e5e030a71e0858a6ba"),
            &steps,
        );
        assert_eq!(
            root,
            hex_to_arr("b734860607317db2da7c040d6b94c92a4f0fdbad973a87134b9c4d64249834e8")
        );
    }

    #[test]
    fn test_format_prev_hash() {
        let res = JobBtc::<bitcoin::Block, RpcReqBody>::format_prev_hash(&U256::from_be_hex(
            "00000000000000000001ebcedce3d84dab04cc80fad12e90270e77a2037907b0",
        ));

        assert_eq!(
            res,
            String::from("037907b0270e77a2fad12e90ab04cc80dce3d84d0001ebce0000000000000000")
        );
    }
}

// #[derive(Serialize_tuple)]
// struct JobParamsBtc {
//     id: u32,
//     prevhash: U256,

// }

// TODO: make job copy sturct for only the header
