use std::{path::PathBuf};

use bitcoin::{hashes::Hash, BlockHash, ScriptBuf};
use bitcoincore_rpc::{
    self,
    bitcoin::{self},
    Auth, RpcApi, bitcoincore_rpc_json::GetBlockTemplateResult,
};
use crypto_bigint::{U256, Encoding};

use crate::{p2p::networking::block::Block};

pub struct BlockFetch<BlockT> {
    pub block: BlockT,
    pub tx_hashes: Vec<[u8; 32]>,
    pub height: u32,
    pub reward: u64,
}

pub trait BlockFetcher<BlockT: Block> : Send + Sync {
    type ErrorT: std::fmt::Display;
    fn new(url: &str) -> Self;
    fn fetch_blocktemplate(
        &self,
        vout: impl Iterator<Item = (ScriptBuf, u64)>,
        prev_p2p_share: U256,
    ) -> Result<BlockFetch<BlockT>, Self::ErrorT>;
    fn submit_block(&self, block: &BlockT) -> Result<(), bitcoincore_rpc::Error>;

    fn fetch_block(&self, hash: &U256) -> Result<BlockT, bitcoincore_rpc::Error>;
}

impl BlockFetcher<bitcoin::Block> for bitcoincore_rpc::Client
where
    bitcoin::Block: Block<BlockTemplateT = GetBlockTemplateResult>,
{
    type ErrorT = bitcoincore_rpc::Error;

    fn new(url: &str) -> Self {
        Self::new(
            url,
            Auth::CookieFile(PathBuf::from("/home/sickguy/.bitcoin/regtest/.cookie")),
        )
        .unwrap()
    }

    fn fetch_blocktemplate(
        &self,
        vout: impl Iterator<Item = (ScriptBuf, u64)>,
        prev_p2p_share: U256,
    ) -> Result<BlockFetch<bitcoin::Block>, bitcoincore_rpc::Error> {
        use bitcoincore_rpc::json::*;

        let header = self.get_block_template(
            GetBlockTemplateModes::Template,
            &[GetBlockTemplateRules::SegWit],
            &[],
        )?;
        let height = header.height as u32;

        let (block, tx_hashes) = bitcoin::Block::from_block_template(&header, vout, prev_p2p_share);

        Ok(BlockFetch {
            block,
            height,
            tx_hashes,
            reward: header.coinbase_value.to_sat(),
        })
    }

    fn fetch_block(&self, hash: &U256) -> Result<bitcoin::Block, bitcoincore_rpc::Error> {
        self.get_block(&BlockHash::from_byte_array(hash.clone().to_be_bytes()))
    }

    fn submit_block(&self, block: &bitcoin::Block) -> Result<(), bitcoincore_rpc::Error> {
        RpcApi::submit_block(self, &block)
    }
}
