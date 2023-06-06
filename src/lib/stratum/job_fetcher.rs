use std::path::PathBuf;

use bitcoincore_rpc::{self, bitcoin::block::Header, Auth, RpcApi};

use crate::stratum::job_btc::BlockHeader;

pub trait HeaderFetcher {
    type HeaderT: BlockHeader + std::fmt::Debug;
    type ErrorT: std::fmt::Display;
    fn fetch_header(&self) -> Result<Self::HeaderT, Self::ErrorT>;
    fn new(url: &str) -> Self;
}

impl HeaderFetcher for bitcoincore_rpc::Client {
    type HeaderT = Header;
    type ErrorT = bitcoincore_rpc::Error;

    fn new(url: &str) -> Self {
        Self::new(
            url,
            Auth::CookieFile(PathBuf::from("/home/sickguy/.bitcoin/regtest/.cookie")),
        )
        .unwrap()
    }

    fn fetch_header(&self) -> Result<Header, bitcoincore_rpc::Error> {
        use bitcoincore_rpc::json::*;

        let header = self.get_block_template(
            GetBlockTemplateModes::Template,
            &[GetBlockTemplateRules::SegWit],
            &[],
        )?;

        Ok(Header::from_block_template(&header))
    }
}

#[cfg(test)]
mod tests {
    use job_btc::BlockHeader;
    use crate::stratum::job_btc;

    use super::HeaderFetcher;
    use bitcoincore_rpc::{self, bitcoin::block::Header};

    struct TestBtcFetcher {}
    impl HeaderFetcher for TestBtcFetcher {
        type HeaderT = Header;
        type ErrorT = bitcoincore_rpc::Error;

        fn new(url: &str) -> Self {
            TestBtcFetcher {}
        }

        fn fetch_header(&self) -> Result<Header, bitcoincore_rpc::Error> {
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
