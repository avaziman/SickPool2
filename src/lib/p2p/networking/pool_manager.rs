use std::path::Path;

pub struct PoolManager {
    pools_dir: Box<Path>,
}

impl PoolManager {
    pub fn new(data_dir: Box<Path>) -> Self {
        let mut data_dir = data_dir.clone().to_path_buf();
        data_dir.push("pools");
        let pools_dir = data_dir.into_boxed_path();

        Self {
            pools_dir,
        }
    }
}
