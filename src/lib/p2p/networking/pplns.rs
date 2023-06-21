// once the pplns window is full it will never empty again,
// so accounting for a non full pplns window state is just adding more bug causing complexity
// thus for simplicity the pplns (and support :) window will start full of dev fee shares, and will never be empty.

use std::collections::{HashMap, VecDeque};

use bitcoincore_rpc::bitcoin::hashes::Hash;
use crypto_bigint::U256;
use serde::{Deserialize, Serialize};

use crate::stratum::header::BlockHeader;

use super::{
    block::Block,
    difficulty::get_diff,
    hard_config::{PPLNS_DIFF_MULTIPLIER, PPLNS_SHARE_UNITS},
    protocol::{Address, ShareP2P},
};

pub type Score = u64;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ScoreChanges {
    pub added: Vec<(Address, Score)>,
    pub removed: Vec<(Address, Score)>,
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

    pub fn verify_scores(&self, score: Score) -> bool {
        let added: Score = self.added.iter().map(|x| x.1).sum();
        let removed: Score = self.removed.iter().map(|x| x.1).sum();

        added == removed && added == score
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
        Self {
            pplns_window: VecDeque::new(),
            pplns_sum: 0,
            oldest_height: 0,
            address_scores: HashMap::new(),
        }
    }

    pub fn add(&mut self, share: ShareP2P<BlockT>, hash: U256, height: u32) -> Result<(), String> {
        let diff = get_diff(hash);

        let entry = WindowEntry {
            score: diff * PPLNS_SHARE_UNITS / get_diff(share.block.get_header().get_target()),
            share,
        };

        if !entry.share.score_changes.verify_scores(entry.score) {
            return Err("Bad scores.".into());
        }

        self.pplns_sum += entry.score;
        self.pplns_window.push_front(entry);

        let mut last_removed = None;

        // clean expired pplns...
        while self.pplns_sum > PPLNS_DIFF_MULTIPLIER {
            let entry = self.pplns_window.pop_back().unwrap();

            self.pplns_sum -= entry.score;
            last_removed = Some(entry);
        }

        // pplns window must always be full.
        let mut last_removed = last_removed.unwrap();

        let remaining = PPLNS_DIFF_MULTIPLIER - self.pplns_sum;
        self.pplns_sum += remaining;
        last_removed.score = remaining;

        self.oldest_height = last_removed.share.encoded.height;
        self.pplns_window.push_back(last_removed);
        Ok(())
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

    fn get_shares(&self) -> VecDeque<WindowEntry<BlockT>> {
        self.pplns_window.clone()
    }
}
