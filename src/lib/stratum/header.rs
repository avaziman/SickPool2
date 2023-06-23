
use bitcoincore_rpc::bitcoin::hashes::Hash;
use bitcoincore_rpc::bitcoin::{Target};

use crypto_bigint::{U256, Encoding};
use serde::{Serialize};
use serde::de::DeserializeOwned;

use super::protocol::SubmitReqParams;


pub trait BlockHeader : Clone + std::fmt::Debug + Serialize + DeserializeOwned {
    type SubmitParams;

    fn get_hash(&self) -> U256;
    fn get_target(&self) -> U256;
    fn get_time(&self) -> u32;
    fn get_prev(&self) -> U256;
    fn get_version(&self) -> u32;
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
        U256::from_le_bytes(self.block_hash().to_byte_array())
    }


    fn get_prev(&self) -> U256 {
        U256::from_le_bytes(self.prev_blockhash.to_byte_array())
    }

    fn get_target(&self) -> U256 {
        U256::from_le_bytes(Target::from_compact(self.bits).to_le_bytes())
    }

    fn equal(&self, other: &Self) -> bool {
        self.prev_blockhash == other.prev_blockhash
    }

    fn get_time(&self) -> u32 {
        self.time
    }

    fn get_version(&self) -> u32 {
        self.version.to_consensus() as u32
    }
    
}
