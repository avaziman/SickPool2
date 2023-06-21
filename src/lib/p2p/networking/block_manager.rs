// only save tip in memory the rest dump on disk
// only keep the blocks of the current window.

use std::io::{Read, Write};
use std::sync::atomic::{AtomicU32, Ordering};
use std::{fs, path::Path};

use super::{block::Block, protocol::ShareP2P};

pub struct BlockManager<T> {
    blocks_dir: Box<Path>,
    tip: ShareP2P<T>,
    current_height: AtomicU32,
}

impl<T: Block> BlockManager<T> {
    pub fn new(data_dir: Box<Path>) -> Self {
        let genesis = ShareP2P::genesis();

        let mut data_dir = data_dir.clone().to_path_buf();
        data_dir.push("shares");

        let blocks_dir = data_dir.into_boxed_path();

        fs::create_dir(&blocks_dir).expect("Failed to create blocks dir");

        Self {
            blocks_dir,
            tip: genesis,
            current_height: AtomicU32::new(0),
        }
    }

    pub fn height(&self) -> u32 {
        self.current_height.load(Ordering::Relaxed)
    }
    pub fn tip(&self) -> &ShareP2P<T> {
        &self.tip
    }

    pub fn new_block(&self, height: u32) {
        self.current_height.store(height, Ordering::Relaxed);
    }

    fn save_share(&self, share: &ShareP2P<T>) -> std::io::Result<()>{
        let path = self.get_block_path(share.encoded.height);

        fs::File::create(path)
            ?
            .write_all(&bincode::serialize(share).unwrap())?;
        Ok(())
    }

    fn load_share(&self, height: u32) -> std::io::Result<T> {
        let path = self.get_block_path(height);

        let mut bytes = Vec::new();
        fs::File::create(path)?.read_to_end(&mut bytes)?;

        match bincode::deserialize(&bytes) {
            Ok(k) => Ok(k),
            Err(e) => Err(std::io::Error::new(
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
