use std::collections::HashMap;

use bitcoin::absolute::LockTime;
use bitcoin::hashes::Hash;
use bitcoin::merkle_tree::{self, calculate_root_inline};

use bitcoin::{OutPoint, ScriptBuf, Sequence, Transaction, TxIn, Witness};
use bitcoincore_rpc::bitcoin::block::Version;

use bitcoincore_rpc::bitcoin::hash_types::TxMerkleNode;

use bitcoincore_rpc::bitcoin::{self, block, CompactTarget, Network, TxOut};
use bitcoincore_rpc::bitcoincore_rpc_json::GetBlockTemplateResult;
use itertools::Itertools;

use crate::stratum::header::BlockHeader;
use crate::stratum::protocol::SubmitReqParams;

use super::block::{Block, EncodeErrorP2P};
use super::hard_config::GENERATION_GRAFFITI;
use super::pplns::{get_reward, get_score, MyBtcAddr, Score, ScoreChanges};
use super::protocol::{Address, CoinabseEncodedP2P, ShareP2P};

// fn compare_outputs(o1: &TxOut, o2: &TxOut) -> bool {
//     o1.value == o2.value && o1.script_pubkey == o2.script_pubkey
// }

pub const COINB1_SIZE: usize = 4 + 1 /* one input */+ 32 + 4 + MIN_SCRIPT_SIZE;
const NETWORK: Network = Network::Bitcoin;

pub const MIN_SCRIPT_SIZE: usize = 4 /* height bytes amount will remain same for 300 years */ + 1 + GENERATION_GRAFFITI.len() + std::mem::size_of::<CoinabseEncodedP2P>() +1 /* push nonce */;

impl Block for block::Block {
    type HeaderT = block::Header;

    type BlockTemplateT = GetBlockTemplateResult;

    fn genesis() -> Self {
        // println!("BITCOIN GENESIS: {:#?}", bitcoin::blockdata::constants::genesis_block(bitcoin::Network::Bitcoin).header);
        bitcoin::blockdata::constants::genesis_block(bitcoin::Network::Bitcoin)
    }

    fn from_block_template(
        template: &GetBlockTemplateResult,
        vout: &HashMap<Address, Score>,
        prev_p2p_share: [u8; 32],
    ) -> (Self, Vec<[u8; 32]>) {
        let output = vout
            .iter()
            .map(|(addr, score)| TxOut {
                value: get_reward(*score, template.coinbase_value.to_sat()),
                script_pubkey: addr.0.script_pubkey(),
            })
            .collect_vec();
        // info!("Outputs: {:?}", output);

        let height = template.height;
        let script = ScriptBuf::builder()
            .push_int(height as i64)
            .push_slice(GENERATION_GRAFFITI)
            // p2p encoded consensus
            .push_slice(prev_p2p_share)
            // space for 8 byte extra nonce1 + extra nonce2
            .push_slice(&0u64.to_le_bytes())
            .into_script();

        // height needs to be > 65,536 for height to take 4 bytes
        // debug_assert_eq!(script.len(), MIN_SCRIPT_SIZE);

        let coinbase_tx = Transaction {
            version: 2,
            lock_time: LockTime::ZERO,
            input: Vec::from([TxIn {
                previous_output: OutPoint::null(),
                script_sig: script,
                sequence: Sequence::max_value(),
                witness: Witness::new(),
            }]),
            output,
        };

        let mut txs = Vec::with_capacity(template.transactions.len() + 1);
        txs.push(coinbase_tx);

        for i in &template.transactions {
            txs.push(bitcoin::consensus::deserialize(&i.raw_tx).unwrap());
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

    fn into_p2p(
        self,
        last_p2p: &ShareP2P<Self>,
        last_scores: &HashMap<MyBtcAddr, u64>,
        main_height: u32,
    ) -> Result<ShareP2P<Self>, EncodeErrorP2P> {
        let gen_tx = &self.txdata[0];

        // std::fs::write(
        //     "tests/sample_first_share.json",
        //     serde_json::to_string_pretty(&self).unwrap(),
        // )
        // .unwrap();
        // panic!();

        let gen_input = &gen_tx.input[0];
        let script = gen_input.script_sig.as_script();
        let other_script = 1
            + bitcoin::consensus::encode::VarInt(main_height as u64).len()
            + 1
            + GENERATION_GRAFFITI.len();

        let p2p_bytes = &script.as_bytes()[other_script..];
        let encoded: CoinabseEncodedP2P = bincode::deserialize(p2p_bytes)?;
        let last_hash = last_p2p.block.get_header().get_hash();

        if encoded.prev_hash != last_hash {
            return Err(EncodeErrorP2P::InvalidPrevHash);
        }

        let current_scores = self.deserialize_rewards()?;
        // let last_scores: HashMap<MyBtcAddr, u64> = last_p2p.block.deserialize_rewards()?;
        let score_changes = ScoreChanges::new(current_scores, last_scores.clone());

        Ok(ShareP2P {
            block: self,
            encoded,
            score_changes,
        })
    }

    // payout = score * block_reward => score = payout / block_reward
    fn deserialize_rewards(&self) -> Result<HashMap<MyBtcAddr, u64>, EncodeErrorP2P> {
        let gen_tx = &self.txdata[0];
        let gen_outs: &Vec<TxOut> = &gen_tx.output;
        let mut res = HashMap::with_capacity(gen_outs.len());

        let gen_reward: u64 = gen_outs.iter().map(|o| o.value).sum();

        for out in gen_outs {
            let score = get_score(out.value, gen_reward);
            let addr = bitcoin::Address::from_script(out.script_pubkey.as_script(), NETWORK)?;

            if let Some(_exists) = res.insert(MyBtcAddr(addr), score) {
                // same address twice is unacceptable! bytes are wasted.
                return Err(EncodeErrorP2P::DuplicateAddress);
            }
        }
        Ok(res)
    }
}

impl From<bitcoin::address::Error> for EncodeErrorP2P {
    fn from(_value: bitcoin::address::Error) -> Self {
        // println!("Address error: {}", _value);
        EncodeErrorP2P::InvalidAddress
    }
}

impl From<bincode::Error> for EncodeErrorP2P {
    fn from(_value: bincode::Error) -> Self {
        EncodeErrorP2P::MissingPrevHash
    }
}

#[cfg(test)]
pub mod tests {
    use std::fs;

    use crate::p2p::networking::{block::Block, pplns::WindowPPLNS, protocol::ShareP2P};

    #[test]
    fn test_into_p2p() {
        let last_share = ShareP2P::<bitcoin::Block>::genesis();
        let candidate: bitcoin::Block =
            serde_json::from_str(&fs::read_to_string("tests/sample_first_share.json").unwrap())
                .unwrap();

        let p2p_share = candidate.into_p2p(
            &last_share,
            &WindowPPLNS::<bitcoin::Block>::new().address_scores,
            1,
        );

        print!("p2pshare {:?}", p2p_share);
        assert_eq!(p2p_share.is_ok(), true);
    }
}
