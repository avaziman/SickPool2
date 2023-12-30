// only save tip in memory the rest dump on disk
// only keep the blocks of the current window.

use std::collections::HashMap;
use std::io::Read;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::{fs, path::Path};

use crate::coins::coin::Coin;
use crate::p2p::networking::block::Block;
use crate::p2p::networking::difficulty::get_diff_score;
use crate::p2p::networking::pplns::ScoreChanges;
use crate::p2p::networking::share::ShareP2P;
use crate::stratum::header::BlockHeader;
use crypto_bigint::U256;
use log::{info, warn};

use crate::p2p::networking::block::EncodeErrorP2P;

use crate::p2p::networking::messages::ShareVerificationError;
use crate::p2p::networking::pplns::{self, Score, WindowPPLNS};

use super::target_manager::TargetManager;

// we don't need the entire block for verification...
pub struct BlockVerifyContext {
    hash: U256,
}

pub struct BlockManager<C: Coin> {
    shares_dir: Box<Path>,
    p2p_tip: Mutex<ProcessedShare<C>>,
    main_tip: Mutex<BlockVerifyContext>,
    current_height: AtomicU32,

    round_start_height: AtomicU32,
    round_num: AtomicU32,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ProcessedShare<C: Coin> {
    pub inner: ShareP2P<C>,
    pub hash: U256,
    pub score: Score,
}

impl<C: Coin> BlockManager<C> {
    pub fn new(genesis: ShareP2P<C>, data_dir: Box<Path>) -> Self {
        // let genesis: ShareP2P<C> = ShareP2P::from_genesis_block(fetcher);

        let mut data_dir = data_dir.clone().to_path_buf();
        data_dir.push("shares");
        let hash = genesis.block.get_header().get_hash();

        let blocks_dir = data_dir.into_boxed_path();

        if let Err(e) = fs::create_dir_all(&blocks_dir) {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                panic!("Failed to create blocks dir: {e}");
            }
        }

        Self {
            shares_dir: blocks_dir,
            main_tip: Mutex::new(BlockVerifyContext {
                hash: genesis.block.get_header().get_hash(),
            }),
            p2p_tip: Mutex::new(ProcessedShare {
                inner: genesis,
                hash,
                // doesnt matter
                score: 0,
            }),
            current_height: AtomicU32::new(0),
            round_start_height: AtomicU32::new(0),
            round_num: AtomicU32::new(0),
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

    // println!("p2p tip hash:  {}", p2p_tip.block.get_header().get_hash());
    // std::fs::write(
    //     "tests/sample_first_share.json",
    //     serde_json::to_string_pretty(&block).unwrap(),
    // )
    // .unwrap();
    // panic!();

    pub fn decode_share(
        block: C::BlockT,
        last_scores: &HashMap<C::Address, u64>,
    ) -> Result<ShareP2P<C>, ShareVerificationError> {
        let current_scores = Self::get_scores(&block);
        let p2p_encoded = block.deserialize_p2p_encoded()?;

        Ok(ShareP2P {
            block,
            encoded: p2p_encoded,
            score_changes: ScoreChanges::new(current_scores, last_scores.clone())?,
        })
    }

    pub fn process_share(
        &self,
        block: C::BlockT,
        p2ptarget: &TargetManager,
        window: &WindowPPLNS<C>,
    ) -> Result<ProcessedShare<C>, ShareVerificationError> {
        let mut p2p_tip = self.p2p_tip.lock().unwrap();

        let p2ptarget = p2ptarget.target();

        
        let share: ShareP2P<C> = Self::decode_share(block, &window.address_scores)?;
        
        let main_hash = self.main_tip().hash;

        // check mainnet link
        if share.block.get_header().get_prev() != main_hash {
            info!("GIVEN PREV: {}", share.block.get_header().get_prev());
            info!("EXP PREV: {}", main_hash);
            return Err(ShareVerificationError::BadLinkMain);
        }

        if !share.block.verify_main_consensus(self.height()) {
            return Err(ShareVerificationError::BadLinkMain);
        }

        // check p2p link
        if share.encoded.prev_hash != p2p_tip.hash
            || share.encoded.height != p2p_tip.inner.encoded.height + 1
            || share.encoded.round_num != p2p_tip.inner.encoded.round_num
        {
            return Err(ShareVerificationError::BadLinkP2P);
        }

        let hash = share.block.get_header().get_hash();
        if &hash > p2ptarget {
            warn!(
                "Insufficient diffiuclty: given {}, target: {}",
                &hash, p2ptarget
            );

            return Err(ShareVerificationError::BadTarget);
        }

        // share score is: share_diff / target_diff
        let score = get_diff_score(&hash, &share.block.get_header().get_target());
        // println!("Share score: {}", score);
        // info!("HASH: {}", hash);

        if window.verify_changes(&share.score_changes, score) {
            warn!("Score changes are unbalanced...");
            return Err(ShareVerificationError::BadRewards);
        }

        let _ = self.save_share(&share);

        let res = ProcessedShare {
            inner: share,
            score,
            hash,
        };
        *p2p_tip = res.clone();
        info!("New p2p tip, score: {}, hash: {}", score, hash);
        self.round_start_height
            .store(self.round_start_height() + 1, Ordering::Relaxed);

        Ok(res)
    }

    pub fn round_start_height(&self) -> u32 {
        self.round_start_height.load(Ordering::Relaxed)
    }

    pub fn round_num(&self) -> u32 {
        self.round_num.load(Ordering::Relaxed)
    }

    pub fn height(&self) -> u32 {
        self.current_height.load(Ordering::Relaxed)
    }
    pub fn p2p_tip(&self) -> MutexGuard<ProcessedShare<C>> {
        self.p2p_tip.lock().unwrap()
    }

    pub fn main_tip(&self) -> MutexGuard<BlockVerifyContext> {
        self.main_tip.lock().unwrap()
    }

    pub fn new_block(&self, height: u32, block_hash: &U256) {
        self.current_height.store(height, Ordering::Relaxed);
        let mut lock = self.main_tip.lock().unwrap();
        *lock = BlockVerifyContext {
            hash: block_hash.clone(),
        };

        info!(
            "New mainchain block: {}, height: {}",
            lock.hash,
            self.height(),
        );
    }

    fn save_share(&self, share: &ShareP2P<C>) -> std::io::Result<()> {
        let path = self.get_share_path(share.encoded.height);

        fs::write(path, &bincode::serialize(&share.block).unwrap())
    }

    fn load_share(&self, height: u32) -> std::io::Result<C::BlockT> {
        let path = self.get_share_path(height);

        let mut bytes = Vec::new();
        fs::File::open(path)?.read_to_end(&mut bytes)?;

        match bincode::deserialize(&bytes) {
            Ok(k) => Ok(k),
            Err(_e) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Failed to deserialize",
            )),
        }
    }

    pub fn load_shares(&self, from_height: u32, count: u8) -> std::io::Result<Vec<C::BlockT>> {
        let mut vec = Vec::new();
        let to = (from_height + count as u32).min(self.height() + 1);
        for i in from_height..to {
            vec.push(self.load_share(i)?);
        }
        Ok(vec)
    }

    fn get_share_path(&self, height: u32) -> Box<Path> {
        let mut path = self.shares_dir.to_path_buf();
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
