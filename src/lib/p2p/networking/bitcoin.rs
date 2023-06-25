use std::collections::HashMap;

use bitcoin::absolute::LockTime;
use bitcoin::merkle_tree::calculate_root_inline;
use bitcoin::psbt::Output;
use bitcoin::{OutPoint, ScriptBuf, Sequence, Transaction, TxIn, Witness};
use bitcoincore_rpc::bitcoin::block::Version;

use bitcoincore_rpc::bitcoin::hash_types::TxMerkleNode;
use bitcoincore_rpc::bitcoin::hashes::Hash;

use bitcoincore_rpc::bitcoin::{self, block, CompactTarget, Network, TxOut};
use bitcoincore_rpc::bitcoincore_rpc_json::GetBlockTemplateResult;
use itertools::Itertools;

use super::block::Block;
use super::hard_config::{GENERATION_GRAFFITI, PPLNS_SHARE_UNITS};
use super::pplns::{MyBtcAddr, Score, ScoreChanges};
use super::protocol::{Address, CoinabseEncodedP2P, ShareP2P};

// fn compare_outputs(o1: &TxOut, o2: &TxOut) -> bool {
//     o1.value == o2.value && o1.script_pubkey == o2.script_pubkey
// }

pub const COINB1_SIZE: usize = 4 + 1 /* one input */+ 32 + 4 + 1 + 4 /* height bytes amount will remain same for 300 years */ + GENERATION_GRAFFITI.len();

impl Block for block::Block {
    type HeaderT = block::Header;

    type BlockTemplateT = GetBlockTemplateResult;

    fn genesis() -> Self {
        bitcoin::blockdata::constants::genesis_block(bitcoin::Network::Bitcoin)
    }

    fn from_block_template(
        template: &GetBlockTemplateResult,
        vout: &HashMap<Address, u64>,
    ) -> (Self, Vec<[u8; 32]>) {
        let output = vout
            .iter()
            .map(|(addr, amount)| TxOut {
                value: *amount,
                script_pubkey: addr.0.script_pubkey(),
            })
            .collect_vec();

        let height = template.height;
        let coinbase_tx = Transaction {
            version: 2,
            lock_time: LockTime::ZERO,
            input: Vec::from([TxIn {
                previous_output: OutPoint::null(),
                script_sig: ScriptBuf::builder()
                    .push_int(height as i64)
                    .push_slice(GENERATION_GRAFFITI)
                    .into_script(),
                sequence: Sequence::max_value(),
                witness: Witness::new(),
            }]),
            output,
        };

        let mut txs = Vec::with_capacity(template.transactions.len() + 1);
        txs.push(coinbase_tx);

        let tx_hashes = txs.iter().map(|t| t.txid()).collect_vec();

        for i in &template.transactions {
            txs.push(bitcoin::consensus::deserialize(&i.raw_tx).unwrap());
        }

        (
            Self {
                header: Self::HeaderT {
                    version: Version::from_consensus(template.version as i32),
                    prev_blockhash: template.previous_block_hash,
                    merkle_root: TxMerkleNode::from_byte_array(
                        calculate_root_inline(&mut tx_hashes.clone())
                            .unwrap()
                            .to_byte_array(),
                    ),
                    time: template.min_time as u32,
                    bits: CompactTarget::from_consensus(u32::from_be_bytes(
                        template.bits.clone().try_into().unwrap(),
                    )),
                    nonce: 0,
                },
                txdata: txs,
            },
            tx_hashes.iter().map(|x| x.to_byte_array()).collect_vec(),
        )
    }

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
            if let Ok(addr) =
                bitcoin::Address::from_script(out.script_pubkey.as_script(), Network::Bitcoin)
            {
                if let Some(_exists) = res.insert(MyBtcAddr(addr), score) {
                    // same address twice is unacceptable! bytes are wasted.
                    return None;
                }
            } else {
                return None;
            }
        }
        Some(res)
    }
}
