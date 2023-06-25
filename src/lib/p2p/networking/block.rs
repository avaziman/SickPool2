use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::stratum::header::BlockHeader;

use super::{
    pplns::Score,
    protocol::{Address, ShareP2P},
};

#[derive(Debug, PartialEq)]
pub enum EncodeErrorP2P {
    // the p2p prev hash is missing
    MissingPrevHash,
    // the p2p prev hash is not the tip
    InvalidPrevHash,
    // two outputs to a single address
    DuplicateAddress,
    // output to an invalid address
    InvalidAddress,
}

pub trait Block: Clone + std::fmt::Debug + Serialize + DeserializeOwned {
    type HeaderT: BlockHeader;
    type BlockTemplateT;
    
    fn genesis() -> Self;
    fn get_header_mut(&mut self) -> &mut Self::HeaderT;
    fn get_header(&self) -> &Self::HeaderT;
    fn from_block_template(
        template: &Self::BlockTemplateT,
        vout: &HashMap<Address, Score>,
        prev_p2p_share: [u8; 32],
    ) -> (Self, Vec<[u8; 32]>);
    // fn verify_coinbase_rewards(&self, shares: &WindowPPLNS<Self>) -> bool;

    fn into_p2p(
        self,
        last_p2p: &ShareP2P<Self>,
        last_scores: &HashMap<Address, u64>,
        height: u32,
    ) -> Result<ShareP2P<Self>, EncodeErrorP2P>;
    fn deserialize_rewards(&self) -> Result<HashMap<Address, Score>, EncodeErrorP2P>;
}
