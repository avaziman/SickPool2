use crypto_bigint::U256;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::stratum::header::BlockHeader;

#[derive(Debug, PartialEq)]
pub enum EncodeErrorP2P {
    // the data encoded inside the coinbase tx is invalid
    InvalidScript,
    // two outputs to a single address
    DuplicateAddress,
    // output to an invalid address
    InvalidAddress,
}
use std::hash::Hash;

use super::share::CoinabaseEncodedP2P;
pub trait Block:
    Clone + PartialEq + std::fmt::Debug + Serialize + DeserializeOwned + Send + Sync
{
    type HeaderT: BlockHeader;
    type BlockTemplateT;
    type Script: Send + Sync + PartialEq + Eq + Hash + Clone;

    fn get_header_mut(&mut self) -> &mut Self::HeaderT;
    fn get_header(&self) -> &Self::HeaderT;
    fn from_block_template(
        template: &Self::BlockTemplateT,
        vout: impl Iterator<Item = (Self::Script, u64)>,
        prev_p2p_share: U256,
    ) -> (Self, Vec<[u8; 32]>);
    fn deserialize_rewards(&self) -> Vec<(Self::Script, u64)>;
        
    fn deserialize_p2p_encoded(&self) -> Result<CoinabaseEncodedP2P, EncodeErrorP2P>;
    fn verify_main_consensus(&self, height: u32) -> bool;

    fn get_coinbase_outs(&self) -> u64;
}
