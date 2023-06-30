// only save tip in memory the rest dump on disk
// only keep the blocks of the current window.

use std::collections::HashMap;
use std::io::Read;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::{fs, path::Path};

use crate::coins::coin::Coin;
use crate::p2p::networking::difficulty::get_diff_score;
use crate::p2p::networking::pplns::ScoreChanges;
use crate::stratum::header::BlockHeader;
use crate::stratum::job_fetcher::BlockFetcher;
use crypto_bigint::U256;
use log::{info, warn};

use super::block::EncodeErrorP2P;

use super::messages::ShareVerificationError;
use super::pplns::{self, Score, WindowPPLNS};

use super::target_manager::TargetManager;
use super::{block::Block, protocol::ShareP2P};

pub struct BlockManager<C: Coin> {
    blocks_dir: Box<Path>,
    p2p_tip: Mutex<ShareP2P<C>>,
    main_tip: Mutex<C::BlockT>,
    current_height: AtomicU32,
}

#[derive(Debug)]
pub struct ProcessedShare<C: Coin> {
    pub inner: ShareP2P<C>,
    pub hash: U256,
    pub score: Score,
}

impl<C: Coin> BlockManager<C> {
    pub fn new(fetcher: &impl BlockFetcher<C::BlockT>, data_dir: Box<Path>) -> Self {
        let genesis: ShareP2P<C> = ShareP2P::fetch_genesis(fetcher);

        let mut data_dir = data_dir.clone().to_path_buf();
        data_dir.push("shares");

        let blocks_dir = data_dir.into_boxed_path();

        if let Err(e) = fs::create_dir(&blocks_dir) {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                panic!("Failed to create blocks dir: {e}");
            }
        }

        Self {
            blocks_dir,
            main_tip: Mutex::new(genesis.block.clone()),
            p2p_tip: Mutex::new(genesis),
            current_height: AtomicU32::new(0),
        }
    }

    fn get_scores(block: &C::BlockT) -> Vec<(<C::BlockT as Block>::Script, Score)> {
        let mut current_rewards = block.deserialize_rewards();
        let total_reward = block.get_coinbase_outs();

        for (_addr, reward) in current_rewards.iter_mut() {
            *reward = pplns::get_score(*reward, total_reward);
        }

        current_rewards
    }

    pub fn process_share(
        &self,
        block: C::BlockT,
        p2ptarget: &TargetManager,
        window: &WindowPPLNS<C>,
    ) -> Result<ProcessedShare<C>, ShareVerificationError> {
        let mut p2p_tip = self.p2p_tip.lock().unwrap();

        let p2ptarget = p2ptarget.target();
        // let share = block.into_p2p(&p2p_tip, &window.address_scores, self.height())?;

        // std::fs::write(
        //     "tests/sample_first_share.json",
        //     serde_json::to_string_pretty(&block).unwrap(),
        // )
        // .unwrap();
        // panic!();

        // if !block.verify_main_consensus(self.height()) {
        //     return Err(ShareVerificationError::BadLinkMain);
        // }

        let p2p_encoded = block.deserialize_p2p_encoded()?;
        let current_scores = Self::get_scores(&block);

        let share: ShareP2P<C> = ShareP2P {
            block,
            encoded: p2p_encoded,
            score_changes: ScoreChanges::new(current_scores, window.address_scores.clone())?,
        };

        // println!("GIVEN PREV: {}", share.block.get_header().get_prev());
        // println!("EXP PREV: {}", self.main_tip.lock().unwrap().get_header().get_hash());
        // check mainnet link
        if share.block.get_header().get_prev()
            != self.main_tip.lock().unwrap().get_header().get_hash()
        {
            return Err(ShareVerificationError::BadLinkMain);
        }

        // check p2p link
        if share.encoded.prev_hash != p2p_tip.block.get_header().get_hash() {
            // genesis doesnt encode anything
            if self.height() != 0 {
                return Err(ShareVerificationError::BadLinkP2P);
            }
        }

        let hash: crypto_bigint::Uint<4> = share.block.get_header().get_hash();
        if &hash > p2ptarget {
            eprintln!(
                "Insufficient diffiuclty: given {}, target: {}",
                &hash, p2ptarget
            );

            return Err(ShareVerificationError::BadTarget);
        }

        // share score is: share_diff / target_diff
        let score = get_diff_score(&hash, &share.block.get_header().get_target());
        println!("Share score: {}", score);
        println!("HASH: {}", hash);

        if window.verify_changes(&share.score_changes, score) {
            warn!("Score changes are unbalanced...");
            return Err(ShareVerificationError::BadRewards);
        }

        *p2p_tip = share.clone();
        info!("New p2p tip, score: {}, hash: {}", score, hash);

        Ok(ProcessedShare {
            inner: share,
            score,
            hash,
        })
    }

    pub fn height(&self) -> u32 {
        self.current_height.load(Ordering::Relaxed)
    }
    pub fn p2p_tip(&self) -> MutexGuard<ShareP2P<C>> {
        self.p2p_tip.lock().unwrap()
    }

    pub fn main_tip(&self) -> MutexGuard<C::BlockT> {
        self.main_tip.lock().unwrap()
    }

    pub fn new_block(&self, height: u32, block: &C::BlockT) {
        self.current_height.store(height, Ordering::Relaxed);
        *self.main_tip.lock().unwrap() = block.clone();

        info!("New mainchain block, height: {}", self.height());
    }

    // fn save_share(&self, share: &ShareP2P<T>) -> std::io::Result<()> {
    //     let path = self.get_block_path(share.);

    //     fs::File::create(path)?.write_all(&bincode::serialize(share).unwrap())
    // }

    fn load_share(&self, height: u32) -> std::io::Result<C::BlockT> {
        let path = self.get_block_path(height);

        let mut bytes = Vec::new();
        fs::File::create(path)?.read_to_end(&mut bytes)?;

        match bincode::deserialize(&bytes) {
            Ok(k) => Ok(k),
            Err(_e) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Failed to deserialize",
            )),
        }
    }

    pub fn load_shares(&self) -> std::io::Result<Vec<C::BlockT>> {
        let mut vec = Vec::new();
        for i in 0..self.height() {
            vec.push(self.load_share(i)?);
        }
        Ok(vec)
    }

    fn get_block_path(&self, height: u32) -> Box<Path> {
        let mut path = self.blocks_dir.to_path_buf();
        path.push(height.to_string());
        path.set_extension("dat");
        path.into_boxed_path()
    }
}

impl From<EncodeErrorP2P> for ShareVerificationError {
    fn from(value: EncodeErrorP2P) -> Self {
        ShareVerificationError::BadEncoding(value)
    }
}
