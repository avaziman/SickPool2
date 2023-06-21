

use std::collections::HashMap;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::stratum::header::BlockHeader;

use super::{protocol::{ShareP2P, Address}, pplns::{WindowPPLNS, Score}};

pub trait Block : Clone + std::fmt::Debug + Serialize + DeserializeOwned{
    type HeaderT: BlockHeader;
    type BlockTemplateT;

    fn genesis() -> Self;
    fn get_header_mut(&mut self) -> &mut Self::HeaderT;
    fn get_header(&self) -> &Self::HeaderT;
    fn from_block_template(template: &Self::BlockTemplateT) -> Self;
    // fn verify_coinbase_rewards(&self, shares: &WindowPPLNS<Self>) -> bool;
    fn into_p2p(self, last_p2p: &ShareP2P<Self>, height: u32) -> Option<ShareP2P<Self>>;
    fn deserialize_rewards(&self) -> Option<HashMap<Address, Score>>;
}
