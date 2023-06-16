use crate::stratum::job_btc::BlockHeader;

use super::protocol::ShareWindow;

pub trait Block {
    type HeaderT: BlockHeader;
    fn verify_coinbase_rewards(&self, shares: ShareWindow<Self::HeaderT>);
}

impl Block for bitcoincore_rpc::bitcoin::block::Block {
    type HeaderT = bitcoincore_rpc::bitcoin::block::Header;

    fn verify_coinbase_rewards(&self, shares: ShareWindow<Self::HeaderT>) {
        let coinbase = match self.coinbase(){
            Some(k) => k,
            None => todo!(),
        };
        let reward = coinbase.output.iter().map(|o| o.value).sum();

        for (i, out) in coinbase.output.iter().enumerate() {

            

            shares.get_reward(i, reward);
        }
    }
    
}