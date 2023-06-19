use serde::{Deserialize, Serialize};
use std::{
    fs,
    net::{SocketAddr},
    path::Path,
};

const DEFAULT_PORT: u16 = 9001;

type UnixMs = u64;

#[derive(Debug, Deserialize, Serialize)]
pub struct Peer {
    pub address: SocketAddr,
    // acknowledged version
    pub last_connection_fail: Option<UnixMs>,

    // runtime variables
    #[serde(skip)]
    pub authorized: Option<u32>,
    pub listening_port: Option<u16>,

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
            }
            Err(e) => {
                let newp = Peer {
                    address,
                    last_connection_fail: None,
                    authorized: None,
                    listening_port: None,
                    connected: true,
                };
                newp.save(path);
                newp
            }
        }
    }

    pub fn load(path: &Path) -> std::io::Result<Self> {
        Ok(serde_json::from_slice(&fs::read(path)?)
            .expect(&format!("Bad peer file at: {}", path.display())))
    }

    pub fn save(&self, path: &Path) {
        fs::write(path, serde_json::to_string_pretty(self).unwrap()).unwrap();
    }
}
