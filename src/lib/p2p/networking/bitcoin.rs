use std::collections::HashMap;


use bitcoincore_rpc::bitcoin::block::Version;

use bitcoincore_rpc::bitcoin::hash_types::TxMerkleNode;
use bitcoincore_rpc::bitcoin::hashes::Hash;

use bitcoincore_rpc::bitcoin::{self, block, CompactTarget, Network, TxOut};
use bitcoincore_rpc::bitcoincore_rpc_json::GetBlockTemplateResult;
use itertools::Itertools;


use super::block::Block;
use super::hard_config::PPLNS_SHARE_UNITS;
use super::pplns::{Score, ScoreChanges, MyBtcAddr};
use super::protocol::{Address, CoinabseEncodedP2P, ShareP2P};

// fn compare_outputs(o1: &TxOut, o2: &TxOut) -> bool {
//     o1.value == o2.value && o1.script_pubkey == o2.script_pubkey
// }

impl Block for block::Block {
    type HeaderT = block::Header;

    type BlockTemplateT = GetBlockTemplateResult;

    fn genesis() -> Self {
        bitcoin::blockdata::constants::genesis_block(bitcoin::Network::Bitcoin)
    }

    fn from_block_template(template: &GetBlockTemplateResult) -> Self {
        Self {
            header: Self::HeaderT {
                version: Version::from_consensus(template.version as i32),
                prev_blockhash: template.previous_block_hash,
                merkle_root: TxMerkleNode::from_raw_hash(Hash::all_zeros()),
                time: template.min_time as u32,
                bits: CompactTarget::from_consensus(u32::from_be_bytes(
                    template.bits.clone().try_into().unwrap(),
                )),
                nonce: 0,
            },
            txdata: template
                .transactions
                .iter()
                .map(|txr| bitcoin::consensus::deserialize(&txr.raw_tx).unwrap())
                .collect_vec(),
        }
    }

    // fn verify_coinbase_rewards(&self, shares: &WindowPPLNS<Self>, share: ShareP2P<Self>) -> bool {
    //     let coinbase = match self.coinbase() {
    //         Some(k) => k,
    //         None => return false,
    //     };

    //     let last_block = shares.pplns_window[0].share.block;
    //     let last_outputs = last_block.txdata[0].output;
    //     let first_old_tx_out = last_outputs[0];

    //     let new_outputs = last_outputs;

    //     let reward: u64 = coinbase.output.iter().map(|o| o.value).sum();

    //     for (old_out, new_out) in last_outputs.iter().zip(new_outputs) {}
    //     // let mut removed_outs = Vec::new();

    //     // let last_outputs_it = last_outputs.iter();
    //     // for i in 0..(coinbase.output.len() - added_outs.len()) {
    //     //     let out = coinbase.output[]
    //     //     let last_out = match last_outputs_it.next() {
    //     //         Some(k) => k,
    //     //         None => return false,
    //     //     };

    //     //     if !compare_outputs(last_out, out) {
    //     //         removed_outs.push(last_out);

    //     //     }
    //     // }

    //     // for (i, out) in coinbase.output.iter().enumerate() {
    //     //     if out.value != shares.get_reward(i, reward) {
    //     //         return false;
    //     //     }
    //     // }

    //     true
    // }

    fn get_header_mut(&mut self) -> &mut Self::HeaderT {
        &mut self.header
    }

    fn get_header(&self) -> &Self::HeaderT {
        &self.header
    }

    fn into_p2p(self, last_p2p: &ShareP2P<Self>, height: u32) -> Option<ShareP2P<Self>> {
        let gen_tx = &self.txdata[0];

        let gen_input = &gen_tx.input[0];
        let script = gen_input.script_sig.as_script();
        let height_script = bitcoin::consensus::encode::VarInt(height as u64).len();

        let p2p_bytes = &script.as_bytes()[height_script..];
        let encoded: CoinabseEncodedP2P = match bincode::deserialize(p2p_bytes) {
            Ok(k) => k,
            Err(_) => return None,
        };

        let current_scores = self.deserialize_rewards()?;
        let last_scores = last_p2p.block.deserialize_rewards()?;
        let score_changes = ScoreChanges::new(current_scores, last_scores);

        Some(ShareP2P {
            block: self,
            encoded,
            score_changes,
        })
    }

    // payout = score * block_reward => score = payout / block_reward
    fn deserialize_rewards(&self) -> Option<HashMap<Address, Score>> {
        let mut res = HashMap::new();
        let gen_tx = &self.txdata[0];
        let gen_outs: &Vec<TxOut> = &gen_tx.output;

        let gen_reward: u64 = gen_outs.iter().map(|o| o.value).sum();

        for out in gen_outs {
            let score = (out.value * PPLNS_SHARE_UNITS) / gen_reward;
            let addr =
                bitcoin::Address::from_script(out.script_pubkey.as_script(), Network::Bitcoin).unwrap();

            if let Some(_exists) = res.insert(MyBtcAddr(addr), score) {
                // same address twice is unacceptable! bytes are wasted.
                return None;
            }
        }
        Some(res)
    }
}
