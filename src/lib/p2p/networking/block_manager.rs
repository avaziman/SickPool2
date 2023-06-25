// only save tip in memory the rest dump on disk
// only keep the blocks of the current window.

use std::io::Read;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::{fs, path::Path};

use crypto_bigint::U256;
use log::{info, warn};

use crate::p2p::networking::difficulty::get_diff_score;
use crate::stratum::header::BlockHeader;

use super::block::EncodeErrorP2P;
use super::difficulty::get_diff1_score;
use super::hard_config::PPLNS_SHARE_UNITS;
use super::messages::ShareVerificationError;
use super::pplns::{Score, WindowPPLNS};

use super::{block::Block, protocol::ShareP2P};

pub struct BlockManager<T> {
    blocks_dir: Box<Path>,
    p2p_tip: Mutex<ShareP2P<T>>,
    main_tip: Mutex<T>,
    current_height: AtomicU32,
}

#[derive(Debug)]
pub struct ProcessedShare<T> {
    pub inner: ShareP2P<T>,
    pub hash: U256,
    pub score: Score,
}

impl<T: Block> BlockManager<T> {
    pub fn new(data_dir: Box<Path>) -> Self {
        let genesis: ShareP2P<T> = ShareP2P::genesis();

        let mut data_dir = data_dir.clone().to_path_buf();
        data_dir.push("shares");

        let blocks_dir = data_dir.into_boxed_path();

        if let Err(e) = fs::create_dir(&blocks_dir) {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                panic!("Failed to create blocks dir");
            }
        }

        Self {
            blocks_dir,
            main_tip: Mutex::new(genesis.block.clone()),
            p2p_tip: Mutex::new(genesis),
            current_height: AtomicU32::new(0),
        }
    }

    pub fn process_share(
        &self,
        block: T,
        p2ptarget: &U256,
        window: &WindowPPLNS<T>,
    ) -> Result<ProcessedShare<T>, ShareVerificationError> {
        let mut p2p_tip = self.p2p_tip.lock().unwrap();

        let share = block.into_p2p(&p2p_tip, &window.address_scores, self.height())?;

        // check mainnet link
        if share.block.get_header().get_prev()
            != self.main_tip.lock().unwrap().get_header().get_prev()
        {
            return Err(ShareVerificationError::BadLinkMain);
        }

        // check p2p link
        if share.encoded.prev_hash != p2p_tip.block.get_header().get_hash() {
            return Err(ShareVerificationError::BadLinkP2P);
        }

        let hash: crypto_bigint::Uint<4> = share.block.get_header().get_hash();
        if &hash > p2ptarget {
            warn!("Insufficient diffiuclty");

            return Err(ShareVerificationError::BadTarget);
        }

        let score = get_diff_score(&hash, &share.block.get_header().get_target());
        // info!("Share score: {}", score);

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
    pub fn p2p_tip(&self) -> MutexGuard<ShareP2P<T>> {
        self.p2p_tip.lock().unwrap()
    }

    pub fn main_tip(&self) -> MutexGuard<T> {
        self.main_tip.lock().unwrap()
    }

    pub fn new_block(&self, height: u32, block: &T) {
        self.current_height.store(height, Ordering::Relaxed);
        *self.main_tip.lock().unwrap() = block.clone();
    }

    // fn save_share(&self, share: &ShareP2P<T>) -> std::io::Result<()> {
    //     let path = self.get_block_path(share.);

    //     fs::File::create(path)?.write_all(&bincode::serialize(share).unwrap())
    // }

    fn load_share(&self, height: u32) -> std::io::Result<T> {
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

    pub fn load_shares(&self) -> std::io::Result<Vec<T>> {
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
