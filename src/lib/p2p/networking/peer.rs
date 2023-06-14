use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    net::{IpAddr, SocketAddr},
    path::Path,
    time::Instant,
};

const DEFAULT_PORT: u16 = 9001;

type UnixMs = u64;

#[derive(Debug, Deserialize, Serialize)]
pub struct Peer {
    pub address: SocketAddr,
    // acknowledged version
    pub last_connection_fail: Option<UnixMs>,
    // runtime variables
    // #[serde(skip)]
    #[serde(skip)]
    pub authorized: bool,
    
    #[serde(default = "bool::default")]
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub connected: bool,
}

impl Peer {
    pub fn new(path: &Path, address: SocketAddr) -> Self {
        match Self::load(path) {
            Ok(mut exists) => {
                // we are about to connect... this method is called on connection.
                exists.connected = true;
                exists.save(path);
                exists
            },
            Err(e) => {
                let newp = Peer {
                    address,
                    last_connection_fail: None,
                    authorized: false,
                    connected: true,
                };
                newp.save(path);
                newp
            }
        }
    }

    pub fn load(path: &Path) -> std::io::Result<Self> {
        // let path = get_path(address, datadir);

        Ok(serde_json::from_reader(fs::File::open(path)?).unwrap())
    }

    pub fn save(&self, path: &Path) {
        fs::write(path, serde_json::to_string_pretty(self).unwrap()).unwrap();
    }

    fn get_path(&self, datadir: &str) -> String {
        get_path(self.address, datadir)
    }
}
fn get_path(address: SocketAddr, datadir: &str) -> String {
    format!("{}/peers/{}.json", datadir, address)
}
