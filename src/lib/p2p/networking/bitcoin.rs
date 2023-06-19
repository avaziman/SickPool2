use bitcoincore_rpc::bitcoin::block::{Header, Version};
use bitcoincore_rpc::bitcoin::consensus::{Decodable, encode};
use bitcoincore_rpc::bitcoin::hash_types::TxMerkleNode;
use bitcoincore_rpc::bitcoin::hashes::Hash;
use bitcoincore_rpc::bitcoin::{self, block, CompactTarget, Transaction};
use bitcoincore_rpc::bitcoincore_rpc_json::GetBlockTemplateResult;
use itertools::Itertools;

use super::block::Block;
use super::protocol::{ShareP2P, ShareWindow, CoinabseEncodedP2P};

impl Block for block::Block {
    type HeaderT = block::Header;

    type BlockTemplateT = GetBlockTemplateResult;

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

    fn verify_coinbase_rewards(&self, shares: &ShareWindow<Self>) -> bool {
        let coinbase = match self.coinbase() {
            Some(k) => k,
            None => return false,
        };

        let reward = coinbase.output.iter().map(|o| o.value).sum();

        for (i, out) in coinbase.output.iter().enumerate() {
            if out.value != shares.get_reward(i, reward) {
                return false;
            }
        }

        true
    }

    fn get_header_mut(&mut self) -> &mut Self::HeaderT {
        &mut self.header
    }

    fn get_header(&self) -> &Self::HeaderT {
        &self.header
    }

    fn into_p2p(self, height: u32) -> Option<ShareP2P<Self>> {
        let gen_tx = &self.txdata[0];
        let gen_input = &gen_tx.input[0];
        let script = gen_input.script_sig.as_script();
        let height_script = bitcoin::consensus::encode::VarInt(height as u64).len();

        let p2p_bytes = &script.as_bytes()[height_script..];
        let encoded : CoinabseEncodedP2P = match bincode::deserialize(p2p_bytes) {
            Ok(k) => k,
            Err(_) => return None,
        };

        Some(ShareP2P {
            block: self,
            encoded,
        })
    }
}
