
use itertools::Itertools;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::stratum::header::BlockHeader;

use super::protocol::{ShareWindow, ShareP2P};

pub trait Block : Clone + std::fmt::Debug + Serialize + DeserializeOwned{
    type HeaderT: BlockHeader;
    type BlockTemplateT;

    fn get_header_mut(&mut self) -> &mut Self::HeaderT;
    fn get_header(&self) -> &Self::HeaderT;
    fn from_block_template(template: &Self::BlockTemplateT) -> Self;
    fn verify_coinbase_rewards(&self, shares: &ShareWindow<Self>) -> bool;
    fn into_p2p(self, height: u32) -> Option<ShareP2P<Self>>;
}
