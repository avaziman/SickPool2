use crypto_bigint::U256;
use serde::{Deserialize, Serialize};

use crate::{address::Address, coins::coin::Coin};

use super::{
    hard_config::{PPLNS_DIFF_MULTIPLIER, PPLNS_SHARE_UNITS},
    pplns::ScoreChanges,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ShareP2P<C: Coin> {
    pub block: C::BlockT,
    pub encoded: CoinbaseEncodedP2P,
    // #[serde(skip)]
    // hash: U256,
    pub score_changes: ScoreChanges<C::Address>,
}

// p2pool prev hash is encoded inside block generation tx
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct CoinbaseEncodedP2P {
    pub prev_hash: U256,
    pub height: u32,
    pub round_num: u32,
}

impl CoinbaseEncodedP2P {
    // static sized
    pub fn bytes(self) -> [u8; std::mem::size_of::<Self>()] {
        bincode::serialize(&self).unwrap().try_into().unwrap()
        // unsafe { std::mem::transmute(self) }
    }
}

// impl<C: Coin> ShareP2P<C> {
//     pub fn from_genesis_block(block: C::BlockT) -> Self {
//         Self {
//             encoded: CoinabaseEncodedP2P {
//                 prev_hash: U256::ZERO,
//             },
//             score_changes: ScoreChanges {
//                 added: Vec::from([(
//                     C::Address::from_string(C::DONATION_ADDRESS).expect("INVALID DEV ADDRESS"),
//                     PPLNS_SHARE_UNITS * PPLNS_DIFF_MULTIPLIER,
//                 )]),
//                 removed: Vec::new(),
//             },
//             block,
//         }
//     }
// }
