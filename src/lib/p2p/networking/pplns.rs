// once the pplns window is full it will never empty again,
// so accounting for a non full pplns window state is just adding more bug causing complexity
// thus for simplicity the pplns (and support :) window will start full of dev fee shares, and will never be empty.

use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::{address::Address, coins::coin::Coin};

use super::{
    block::EncodeErrorP2P,
    block_manager::ProcessedShare,
    hard_config::{PPLNS_DIFF_MULTIPLIER, PPLNS_SHARE_UNITS}, share::ShareP2P,
};

pub type Score = u64;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct ScoreChanges<Address> {
    pub added: Vec<(Address, Score)>,
    pub removed: Vec<(Address, Score)>,
}

impl<A: Address> ScoreChanges<A> {
    pub fn new(
        current_scores: Vec<(A::FromScript, u64)>,
        mut last_scores: HashMap<A, u64>,
    ) -> Result<ScoreChanges<A>, EncodeErrorP2P> {
        let mut removed = Vec::new();
        let mut added = Vec::new();
        let mut new_addrs = 0;
        let exp_new_addrs = current_scores.len() - last_scores.len();

        for (key, score) in current_scores.into_iter() {
            let addr = match A::from_script(&key) {
                Ok(k) => k,
                Err(_e) => return Err(EncodeErrorP2P::InvalidAddress),
            };

            match last_scores.remove(&addr) {
                Some(last_score) => {
                    if score > last_score {
                        added.push((addr, score - last_score));
                    } else if last_score > score {
                        removed.push((addr, last_score - score));
                    }
                }
                None => {
                    // completely new, or it has already been removed then its dup
                    added.push((addr, score));
                    new_addrs += 1;
                }
            }
        }

        if new_addrs != exp_new_addrs {
            // same address twice is unacceptable! bytes are wasted.
            return Err(EncodeErrorP2P::DuplicateAddress);
        }

        Ok(ScoreChanges { added, removed })
    }
}

pub struct WindowPPLNS<C: Coin> {
    pub pplns_window: VecDeque<WindowEntry<C>>, // hash, score
    // all shares since last block was found, used to bootstrap and as an height index
    pub address_scores: HashMap<C::Address, Score>,
    pub oldest_height: u32,
    pplns_sum: Score,
}

#[derive(Clone)]
pub struct WindowEntry<C: Coin> {
    pub share: ShareP2P<C>,
    pub score: Score,
}
// pub static PPLNS_DIFF_MULTIPLIER_DECIMAL: Decimal =PPLNS_DIFF_MULTIPLIER.into();

pub fn get_reward(score: Score, total_reward: u64) -> u64 {
    score * total_reward / (PPLNS_DIFF_MULTIPLIER * PPLNS_SHARE_UNITS)
}

pub fn get_score(rewarded: u64, total_reward: u64) -> u64 {
    (rewarded * PPLNS_DIFF_MULTIPLIER * PPLNS_SHARE_UNITS) / total_reward
}

impl<C: Coin> WindowPPLNS<C> {
    pub fn new(genesis: ShareP2P<C>) -> Self {
        let genesis_entry = WindowEntry {
            share: genesis,
            score: PPLNS_SHARE_UNITS * PPLNS_DIFF_MULTIPLIER,
        };

        let mut me = Self {
            pplns_window: VecDeque::new(),
            pplns_sum: 0,
            oldest_height: 0,
            address_scores: HashMap::new(),
        };

        me.add_entry(genesis_entry);
        me
    }

    fn add_entry(&mut self, entry: WindowEntry<C>) {
        self.pplns_sum += entry.score;
        self.add_scores(&entry.share.score_changes.added);
        self.pplns_window.push_front(entry);
    }

    pub fn add(&mut self, pshare: ProcessedShare<C>) {
        let entry = WindowEntry {
            score: pshare.score,
            share: pshare.inner,
        };

        self.remove_scores(&entry.share.score_changes.removed);
        self.add_entry(entry);

        // clean expired pplns...
        // pplns window must always be full.
        loop {
            let entry = self.pplns_window.pop_back().unwrap();

            if self.pplns_sum - entry.score > PPLNS_DIFF_MULTIPLIER * PPLNS_SHARE_UNITS {
                self.pplns_sum -= entry.score;
            } else {
                let remaining = self.pplns_sum - PPLNS_DIFF_MULTIPLIER * PPLNS_SHARE_UNITS;
                self.pplns_window.push_back(WindowEntry {
                    share: entry.share,
                    score: remaining,
                });
                self.pplns_sum -= remaining;

                break;
            }
        }

        // self.oldest_height = last_removed.share.encoded.height;
        debug_assert_eq!(self.pplns_sum, PPLNS_DIFF_MULTIPLIER * PPLNS_SHARE_UNITS);
    }

    pub fn verify_changes(&self, changes: &ScoreChanges<C::Address>, score: Score) -> bool {
        let added: Score = changes.added.iter().map(|x| x.1).sum();
        let removed: Score = changes.removed.iter().map(|x| x.1).sum();

        if added != removed || added != score {
            return false;
        }

        let mut remove_left = added;
        let mut expected_removed = HashMap::new();
        for last_score in self.pplns_window.iter().rev() {
            for (added_to, amt) in &last_score.share.score_changes.added {
                match expected_removed.get_mut(added_to) {
                    Some(k) => {
                        *k += std::cmp::min(amt, &remove_left);
                        remove_left -= amt;
                    }
                    None => {
                        let _x = expected_removed.insert(added_to, *amt);
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

    // called after adding.
    fn remove_scores(&mut self, scores: &Vec<(C::Address, Score)>) {
        for (added_to, amt) in scores {
            *self.address_scores.get_mut(&added_to).unwrap() -= amt;
        }
    }

    fn add_scores(&mut self, scores: &Vec<(C::Address, Score)>) {
        for (added_to, amt) in scores {
            match self.address_scores.get_mut(&added_to) {
                Some(k) => *k += amt,
                None => {
                    self.address_scores.insert(added_to.clone(), *amt);
                }
            };
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

// #[cfg(test)]
// pub mod tests {
//     use std::path::{Path, PathBuf};

//     use bitcoincore_rpc::bitcoin;
//     use crypto_bigint::U256;

//     use crate::p2p::networking::{
//         block::{self, Block},
//         block_manager::{ProcessedShare, BlockManager},
//         messages::ShareVerificationError,
//         protocol::ShareP2P,
//     };

//     #[test]
//     pub fn parse_genesis_main() {
//         let block_manager = BlockManager::new(PathBuf::from("/test/").into_boxed_path());
//         let res = block_manager.process_share(block, p2ptarget, window)(
//             bitcoin::Block::genesis(),
//             &ShareP2P::<bitcoin::Block>::genesis(),
//             0,
//             &U256::ZERO,
//         );

//         assert_eq!(res.err().unwrap(), ShareVerificationError::BadEncoding);
//     }

//     #[test]
//     pub fn parse_genesis_p2p() {
//         let res = ProcessedShare::process(
//             ShareP2P::<bitcoin::Block>::genesis().block,
//             &ShareP2P::<bitcoin::Block>::genesis(),
//             0,
//             &U256::ZERO,
//         );

//         assert_eq!(res.err().unwrap(), ShareVerificationError::BadEncoding);
//     }
// }
