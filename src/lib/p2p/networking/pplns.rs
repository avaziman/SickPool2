// once the pplns window is full it will never empty again,
// so accounting for a non full pplns window state is just adding more bug causing complexity
// thus for simplicity the pplns (and support :) window will start full of dev fee shares, and will never be empty.

use std::collections::{HashMap, VecDeque};

use bitcoin::address::NetworkUnchecked;
use bitcoin::Network;

use serde::{Deserialize, Serialize};

use super::{
    block::Block,
    block_manager::ProcessedShare,
    hard_config::{PPLNS_DIFF_MULTIPLIER, PPLNS_SHARE_UNITS},
    protocol::{Address, ShareP2P},
};

pub type Score = u64;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ScoreChanges {
    pub added: Vec<(Address, Score)>,
    pub removed: Vec<(Address, Score)>,
}

#[derive(Serialize, Clone, Debug, Eq, Hash, PartialEq)]
pub struct MyBtcAddr(pub bitcoin::Address);

impl<'de> Deserialize<'de> for MyBtcAddr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(MyBtcAddr(
            bitcoin::Address::<NetworkUnchecked>::deserialize(deserializer)?
                .require_network(Network::Bitcoin)
                .unwrap(),
        ))
    }
}

impl ScoreChanges {
    pub fn new(
        current_scores: HashMap<Address, u64>,
        mut last_scores: HashMap<Address, u64>,
    ) -> ScoreChanges {
        let mut removed = Vec::new();
        let mut added = Vec::new();

        for (key, score) in last_scores.drain() {
            match current_scores.get(&key) {
                Some(last_score) => {
                    let last_score = *last_score;

                    if score > last_score {
                        added.push((key, score - last_score));
                    } else if last_score > score {
                        removed.push((key, last_score - score));
                    }
                }
                None => {
                    // completely removed
                    removed.push((key, score));
                }
            }
        }
        ScoreChanges { added, removed }
    }
}

pub struct WindowPPLNS<BlockT> {
    pub pplns_window: VecDeque<WindowEntry<BlockT>>, // hash, score
    // all shares since last block was found, used to bootstrap and as an height index
    pub address_scores: HashMap<Address, Score>,
    pub oldest_height: u32,
    pplns_sum: Score,
}

#[derive(Clone)]
pub struct WindowEntry<T> {
    pub share: ShareP2P<T>,
    pub score: Score,
}
// pub static PPLNS_DIFF_MULTIPLIER_DECIMAL: Decimal =PPLNS_DIFF_MULTIPLIER.into();

impl<BlockT> WindowPPLNS<BlockT>
where
    BlockT: Block,
{
    pub fn new() -> Self {
        let genesis_entry = WindowEntry {
            share: ShareP2P::<BlockT>::genesis(),
            score: PPLNS_SHARE_UNITS * PPLNS_DIFF_MULTIPLIER,
        };

        Self {
            pplns_window: VecDeque::from([genesis_entry]),
            pplns_sum: 0,
            oldest_height: 0,
            address_scores: HashMap::new(),
        }
    }

    pub fn add(&mut self, pshare: ProcessedShare<BlockT>) {
        let entry = WindowEntry {
            score: pshare.score,
            share: pshare.inner,
        };

        self.pplns_sum += entry.score;
        self.add_scores(&entry.share.score_changes.added);
        self.remove_scores(&entry.share.score_changes.removed);
        self.pplns_window.push_front(entry);

        // clean expired pplns...
        // pplns window must always be full.
        loop {
            let entry = self.pplns_window.pop_back().unwrap();

            self.pplns_sum -= entry.score;

            if self.pplns_sum - entry.score > PPLNS_DIFF_MULTIPLIER {
                break;
            }
        }

        // self.oldest_height = last_removed.share.encoded.height;
        debug_assert_eq!(self.pplns_sum, PPLNS_DIFF_MULTIPLIER * PPLNS_SHARE_UNITS);
    }

    pub fn verify_changes(&self, changes: &ScoreChanges, score: Score) -> bool {
        let added: Score = changes.added.iter().map(|x| x.1).sum();
        let removed: Score = changes.removed.iter().map(|x| x.1).sum();

        if added != removed || added != score {
            return false;
        }

        let mut remove_left = added;
        let mut expected_removed: HashMap<&MyBtcAddr, u64> = HashMap::new();
        for last_score in self.pplns_window.iter().rev() {
            for (added_to, amt) in &last_score.share.score_changes.added {
                match expected_removed.get_mut(added_to) {
                    Some(k) => {
                        *k += std::cmp::min(amt, &remove_left);
                        remove_left -= amt;
                    }
                    None => {
                        let x = expected_removed.insert(&added_to, *amt);
                    }
                }
                if remove_left <= 0 {
                    break;
                }
            }
        }

        if expected_removed.len() != changes.removed.len() {
            return false;
        }

        for (removed, amt) in &changes.removed {
            if let Some(amt2) = expected_removed.get(&removed) {
                if amt != amt2 {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
    // fn add_n_scores(&mut self, n: u64, scores: &Vec<(Address, Score)>) {
    //     let mut total = 0;
    //     for (added_to, amt) in scores {
    //         *self.address_scores.get_mut(&added_to).unwrap() += amt;
    //         total += amt;

    //         if total >= n {
    //             let over = total - n;
    //             *self.address_scores.get_mut(&added_to).unwrap() -= over;
    //         }
    //     }
    // }

    // fn remove_n_scores(&mut self, n: u64, scores: &Vec<(Address, Score)>) {
    //     let mut total = 0;
    //     for (added_to, amt) in scores {
    //         *self.address_scores.get_mut(&added_to).unwrap() -= amt;
    //         total += amt;

    //         if total >= n {
    //             let over = total - n;
    //             *self.address_scores.get_mut(&added_to).unwrap() += over;
    //         }
    //     }
    // }

    fn remove_scores(&mut self, scores: &Vec<(Address, Score)>) {
        for (added_to, amt) in scores {
            *self.address_scores.get_mut(&added_to).unwrap() -= amt;
        }
    }

    fn add_scores(&mut self, scores: &Vec<(Address, Score)>) {
        for (added_to, amt) in scores {
            *self.address_scores.get_mut(&added_to).unwrap() += amt;
        }
    }

    pub fn get_modified_pplns(&self) {}

    // pub fn get_reward(&self, i: usize, reward: u64) -> u64 {
    //     let share = &self.pplns_window[i];

    //     // (reward * get_diff(hash)) / self.sum
    //     // (Decimal::new(reward as i64, 0) * share.score)
    //     //     .to_u64()
    //     //     .unwrap()
    // }

    // pub fn clean_expired(&mut self, current_height: u32) {
    //     // while let Some(back) = self.window.back() {
    //     //     if !is_eligble_to_submit(back.2, current_height) {
    //     //         self.window.pop_back();
    //     //     } else {
    //     //         break;
    //     //     }
    //     // }
    // }
}

#[cfg(test)]
pub mod tests {
    // use bitcoincore_rpc::bitcoin;
    // use crypto_bigint::U256;

    // use crate::p2p::networking::{
    //     block::{self, Block},
    //     messages::ShareVerificationError,
    //     protocol::{ProcessedShare, ShareP2P},
    // };

    // #[test]
    // pub fn parse_genesis_main() {
    //     let res = ProcessedShare::process(
    //         bitcoin::Block::genesis(),
    //         &ShareP2P::<bitcoin::Block>::genesis(),
    //         0,
    //         &U256::ZERO,
    //     );

    //     assert_eq!(res.err().unwrap(), ShareVerificationError::BadEncoding);
    // }

    // #[test]
    // pub fn parse_genesis_p2p() {
    //     let res = ProcessedShare::process(
    //         ShareP2P::<bitcoin::Block>::genesis().block,
    //         &ShareP2P::<bitcoin::Block>::genesis(),
    //         0,
    //         &U256::ZERO,
    //     );

    //     assert_eq!(res.err().unwrap(), ShareVerificationError::BadEncoding);
    // }
}
