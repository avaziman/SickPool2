
use bitcoincore_rpc::bitcoin::hashes::Hash;
use bitcoincore_rpc::bitcoin::{BlockHash, Target};
use bitcoincore_rpc::json::GetBlockTemplateResult;
use primitive_types::U256;
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;

use super::protocol::SubmitReqParams;


pub trait BlockHeader : Clone + std::fmt::Debug + Serialize + DeserializeOwned {
    type SubmitParams;

    fn get_hash(&self) -> U256;
    fn get_target(&self) -> U256;
    fn get_time(&self) -> u32;
    fn update_fields(&mut self, params: &Self::SubmitParams);
    fn equal(&self, other: &Self) -> bool;
}

impl BlockHeader for bitcoincore_rpc::bitcoin::block::Header {
    // type BlockHashT = BlockHash;
    type SubmitParams = SubmitReqParams;

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

    fn get_time(&self) -> u32 {
        self.time
    }

    
}
