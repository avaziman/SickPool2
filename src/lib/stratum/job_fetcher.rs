use std::path::PathBuf;

use bitcoincore_rpc::{self, bitcoin::{block::Header, self}, Auth, RpcApi};

use crate::{stratum::header::BlockHeader, p2p::networking::block::Block};

pub trait BlockFetcher {
    type BlockT: Block;
    type ErrorT: std::fmt::Display;
    fn fetch_block(&self) -> Result<(Self::BlockT, u32), Self::ErrorT>;
    fn new(url: &str) -> Self;
}

impl BlockFetcher for bitcoincore_rpc::Client {
    type BlockT = bitcoin::Block;
    type ErrorT = bitcoincore_rpc::Error;

    fn new(url: &str) -> Self {
        Self::new(
            url,
            Auth::CookieFile(PathBuf::from("/home/sickguy/.bitcoin/regtest/.cookie")),
        )
        .unwrap()
    }

    fn fetch_block(&self) -> Result<(bitcoin::Block, u32), bitcoincore_rpc::Error> {
        use bitcoincore_rpc::json::*;

        let header = self.get_block_template(
            GetBlockTemplateModes::Template,
            &[GetBlockTemplateRules::SegWit],
            &[],
        )?;
        let height = header.height;

        Ok((Self::BlockT::from_block_template(&header), height as u32))
    }
}

#[cfg(test)]
mod tests {
    use header::BlockHeader;
    use crate::stratum::header;

    use super::BlockFetcher;
    use bitcoincore_rpc::{self, bitcoin::block::Header};

    struct TestBtcFetcher {}
    impl BlockFetcher for TestBtcFetcher {
        type BlockT = Header;
        type ErrorT = bitcoincore_rpc::Error;

        fn new(url: &str) -> Self {
            TestBtcFetcher {}
        }

        fn fetch_block(&self) -> Result<Header, bitcoincore_rpc::Error> {
            use bitcoincore_rpc::json::*;

            let header : GetBlockTemplateResult= serde_json::from_str(
                r#"{
            "capabilities": [
                "proposal"
            ],
            "version": 536870912,
            "rules": [
                "csv",
                "!segwit",
                "taproot"
            ],
            "vbavailable": {
            },
            "vbrequired": 0,
            "previousblockhash": "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206",
            "transactions": [
            ],
            "coinbaseaux": {
            },
            "coinbasevalue": 5000000000,
            "longpollid": "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e22060",
            "target": "7fffff0000000000000000000000000000000000000000000000000000000000",
            "mintime": 1296688603,
            "mutable": [
                "time",
                "transactions",
                "prevblock"
            ],
            "noncerange": "00000000ffffffff",
            "sigoplimit": 80000,
            "sizelimit": 4000000,
            "weightlimit": 4000000,
            "curtime": 1686069956,
            "bits": "207fffff",
            "height": 1,
            "default_witness_commitment": "6a24aa21a9ede2f61c3f71d1defd3fa999dfa36953755c690689799962b48bebd836974e8cf9"
            }"#
         ).unwrap();

            Ok(Header::from_block_template(&header))
        }
    }
}
