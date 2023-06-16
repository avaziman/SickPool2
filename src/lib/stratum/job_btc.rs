use bitcoincore_rpc::bitcoin::block::{Header, Version};
use bitcoincore_rpc::bitcoin::hash_types::TxMerkleNode;
use bitcoincore_rpc::bitcoin::hashes::Hash;
use bitcoincore_rpc::bitcoin::{BlockHash, CompactTarget, Target};
use bitcoincore_rpc::json::GetBlockTemplateResult;
use primitive_types::U256;
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;

use super::protocol::SubmitReqParams;


pub trait BlockHeader : Clone + std::fmt::Debug + Serialize + DeserializeOwned {
    type BlockTemplateT;
    type SubmitParams;

    fn from_block_template(template: &Self::BlockTemplateT) -> Self;
    fn get_hash(&self) -> U256;
    fn get_target(&self) -> U256;
    fn update_fields(&mut self, params: &Self::SubmitParams);
    fn equal(&self, other: &Self) -> bool;
}

impl BlockHeader for bitcoincore_rpc::bitcoin::block::Header {
    type BlockTemplateT = GetBlockTemplateResult;
    // type BlockHashT = BlockHash;
    type SubmitParams = SubmitReqParams;

    fn from_block_template(template: &GetBlockTemplateResult) -> Header {
        Header {
            version: Version::from_consensus(template.version as i32),
            prev_blockhash: template.previous_block_hash,
            merkle_root: TxMerkleNode::from_raw_hash(Hash::all_zeros()),
            time: template.min_time as u32,
            bits: CompactTarget::from_consensus(u32::from_be_bytes(
                template.bits.clone().try_into().unwrap(),
            )),
            nonce: 0,
        }
    }

    fn update_fields(&mut self, params: &SubmitReqParams) {
        self.nonce = params.nonce;
    }

    fn get_hash(&self) -> U256 {
        U256::from(self.block_hash().to_byte_array())
    }

    fn get_target(&self) -> U256 {
        U256::from(Target::from_compact(self.bits).to_le_bytes())
    }

    fn equal(&self, other: &Self) -> bool {
        self.prev_blockhash == other.prev_blockhash
    }

    
}
