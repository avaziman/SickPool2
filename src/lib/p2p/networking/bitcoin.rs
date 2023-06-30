use std::collections::HashMap;

use bitcoin::absolute::LockTime;
use bitcoin::hashes::Hash;
use bitcoin::merkle_tree::calculate_root_inline;

use bitcoin::opcodes::all::OP_PUSHNUM_4;
use bitcoin::{OutPoint, ScriptBuf, Sequence, Transaction, TxIn, Witness};
use bitcoincore_rpc::bitcoin::block::Version;

use bitcoincore_rpc::bitcoin::hash_types::TxMerkleNode;

use bitcoincore_rpc::bitcoin::{self, CompactTarget, Network, TxOut};
use bitcoincore_rpc::bitcoincore_rpc_json::GetBlockTemplateResult;
use crypto_bigint::U256;
use itertools::Itertools;
use log::{info, warn};

use crate::coins::bitcoin::{Btc, MyBtcAddr};

use super::block::{Block, EncodeErrorP2P};
use super::hard_config::GENERATION_GRAFFITI;
use super::pplns::{get_reward, Score};
use super::protocol::CoinabseEncodedP2P;
// fn compare_outputs(o1: &TxOut, o2: &TxOut) -> bool {
//     o1.value == o2.value && o1.script_pubkey == o2.script_pubkey
// }

pub const SCRIPTLESS_COINB1_SIZE: usize = 4 + 1 /* one input */+ 32 + 4;
// pub const MIN_SCRIPT_SIZE: usize = 4 /* height bytes amount will remain same for 300 years */ + 1 + GENERATION_GRAFFITI.len() + std::mem::size_of::<CoinabseEncodedP2P>() +1 /* push nonce */;

impl/* <Address: BtcScriptAddr> */ Block for bitcoin::block::Block {
    type HeaderT = bitcoin::block::Header;
    type BlockTemplateT = GetBlockTemplateResult;
    type Script = ScriptBuf;
    
    fn from_block_template(
        template: &GetBlockTemplateResult,
        vout: impl Iterator<Item = (ScriptBuf, u64)>,
        prev_p2p_share: [u8; 32],
    ) -> (Self, Vec<[u8; 32]>) {
        let output = vout
            .map(|(script, score)| TxOut {
                value: get_reward(score, template.coinbase_value.to_sat()),
                script_pubkey: script.clone(),
            })
            .collect_vec();
        // info!("Outputs: {:?}", output);

        let height = template.height;
        let script_sig = generate_bitcoin_script(height, &prev_p2p_share);

        let coinbase_tx = Transaction {
            version: 2,
            lock_time: LockTime::ZERO,
            input: Vec::from([TxIn {
                previous_output: OutPoint::null(),
                sequence: Sequence::max_value(),
                witness: Witness::new(),
                script_sig,
            }]),
            output,
        };

        let mut txs = Vec::with_capacity(template.transactions.len() + 1);
        txs.push(coinbase_tx);

        for i in &template.transactions {
            match i.transaction() {
                Ok(tx) => txs.push(tx),
                Err(err) => warn!(
                    "Failed to deserialize transaction with txid: {} -> {}",
                    &i.txid, err
                ),
            }
        }

        let tx_hashes = txs.iter().map(|t| t.txid()).collect_vec();
        let merkle_root = calculate_root_inline(&mut tx_hashes.clone()).unwrap();

        let merkle_root = TxMerkleNode::from_raw_hash(merkle_root.to_raw_hash());
        let bits_bytes = template.bits.clone().try_into().unwrap();
        // println!("MERKLE ROOT: {}", merkle_root);
        // println!("coinbase txid: {}", tx_hashes[0]);

        (
            Self {
                header: Self::HeaderT {
                    version: Version::from_consensus(template.version as i32),
                    prev_blockhash: template.previous_block_hash,
                    merkle_root,
                    time: template.current_time as u32,
                    bits: CompactTarget::from_consensus(u32::from_be_bytes(bits_bytes)),
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
    // mainnet consensus
    fn verify_main_consensus(&self, check_height: u32) -> bool {
        let gen_tx = &self.txdata[0];
        let gen_input = &gen_tx.input[0];

        if !self.check_merkle_root() {
            return false;
        }

        // regtest doesnt encode height
        if Btc::NETWORK != Network::Regtest {
            let height_script = ScriptBuf::builder()
                .push_int(check_height as i64)
                .into_script();

            gen_input
                .script_sig
                .as_bytes()
                .starts_with(height_script.as_bytes())
        } else {
            true
        }
    }

    // payout = score * block_reward => score = payout / block_reward
    fn deserialize_rewards(&self) -> Vec<(ScriptBuf, u64)> {
        let gen_tx = &self.txdata[0];
        let gen_outs: &Vec<TxOut> = &gen_tx.output;
        let mut res = Vec::with_capacity(gen_outs.len());

        for out in gen_outs {
            let val = out.value;
            res.push((out.script_pubkey.clone(), val));
        }

        res
    }

    // must be called after consensus verified
    fn deserialize_p2p_encoded(&self) -> Result<CoinabseEncodedP2P, EncodeErrorP2P> {
        let mut prev_hash_push = self.txdata[0].input[0].script_sig.instructions();
        // height, must already be verified
        prev_hash_push.next();

        if let Some(prev_hash_push) = prev_hash_push.next() {
            if let Ok(prev_hash_push) = prev_hash_push {
                match prev_hash_push {
                    bitcoin::script::Instruction::PushBytes(bytes) => {
                        return Ok(CoinabseEncodedP2P {
                            prev_hash: U256::from_le_slice(bytes.as_bytes()),
                        })
                    }
                    _ => {}
                }
            }
        }

        Err(EncodeErrorP2P::InvalidScript)
    }

    fn get_coinbase_outs(&self) -> u64 {
        self.txdata[0].output.iter().map(|x| x.value).sum()
    }
}

impl From<bitcoin::address::Error> for EncodeErrorP2P {
    fn from(_value: bitcoin::address::Error) -> Self {
        // println!("Address error: {}", _value);
        EncodeErrorP2P::InvalidAddress
    }
}

fn generate_bitcoin_script(main_height: u64, prev_p2p_share: &[u8; 32]) -> ScriptBuf {
    ScriptBuf::builder()
        .push_int(main_height as i64)
        .push_slice(GENERATION_GRAFFITI)
        // p2p encoded consensus
        .push_slice(prev_p2p_share)
        .push_slice(&0u64.to_le_bytes())
        .into_script()
}

#[cfg(test)]
pub mod tests {
    use std::{fs, path::PathBuf, time::Duration};

    use crypto_bigint::U256;

    use crate::{
        p2p::networking::{
            block::Block,
            block_manager::{BlockManager, ProcessedShare},
            hard_config::{PPLNS_DIFF_MULTIPLIER, PPLNS_SHARE_UNITS},
            pplns::{ScoreChanges, WindowPPLNS},
            protocol::{CoinabseEncodedP2P, ShareP2P},
            target_manager::TargetManager,
        },
        stratum::header::BlockHeader,
    };

    // #[test]
    // fn serialize_first_share_p2p() {}

    #[test]
    fn process_first_share_p2p() {
        let candidate: bitcoin::Block =
            serde_json::from_str(&fs::read_to_string("tests/sample_first_share.json").unwrap())
                .unwrap();
        // hash is 00000039B7B1072EAA7DCB04206600A4FA032DEB13996911679D3AE17F8C395A
        // mill diff is 1732 //.5489874738635

        let target_man = TargetManager::new::<bitcoin::Block>(Duration::from_secs(1), 10);
        let block_manager = BlockManager::new(PathBuf::from("tests/").into_boxed_path());
        let res = block_manager.process_share(
            candidate.clone(),
            &target_man,
            &WindowPPLNS::<bitcoin::Block>::new(),
        );

        // print!("p2pshare {:?}", p2p_share);
        assert_eq!(
            res,
            Ok(ProcessedShare {
                inner: ShareP2P {
                    block: candidate.clone(),
                    encoded: CoinabseEncodedP2P {
                        prev_hash: U256::ZERO
                    },
                    score_changes: ScoreChanges {
                        added: Vec::new(),
                        removed: Vec::new()
                    }
                },
                hash: candidate.get_header().get_hash(),
                score: 1732
            })
        );
    }
}
