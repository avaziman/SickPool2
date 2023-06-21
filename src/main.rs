#![deny(unsafe_code)]
mod currency;

use log::info;
use log4rs;
use sickpool2lib::config::ProtocolServerConfig;
use sickpool2lib::p2p::networking::protocol::ConfigP2P;
use sickpool2lib::p2p::networking::server::ServerP2P;
use sickpool2lib::stratum::server::StratumServer;
use std::path::{Path, PathBuf};
use std::result::Result;
use std::{env, fs};

extern crate sickpool2lib;

use sickpool2lib::stratum::config::StratumConfig;

// type StratumV1Json = JsonRpcProtocol<StratumV1<bitcoincore_rpc::Client>, StratumV1ErrorCodes>;
fn read_config<T: Default + serde::de::DeserializeOwned + serde::Serialize>(
    path: &Box<Path>,
) -> Result<T, String> {
    let data = match fs::read_to_string(path) {
        Ok(k) => k,
        Err(e) => {
            fs::write(
                path,
                serde_json::to_string_pretty(&T::default())
                    .unwrap()
                    .as_bytes(),
            )
            .expect("Failed to write config default file");
            return Err(format!("Missing config file at: {}, generated default config, modify it and restart to continue. {}", path.display(), e));
        }
    };
    match serde_json::from_str(&data) {
        Ok(k) => Ok(k),
        Err(e) => Err(format!("Invalid config at: {}, {}", path.display(), e)),
    }
}

fn main() -> Result<(), String> {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    let args: Vec<String> = env::args().collect();
    let data_dir = if args.len() > 1 {
        args[1].clone()
    } else {
        String::from("datadir")
    };

    let buf = PathBuf::from(&data_dir);

    let stratum_cfg_path =
        PathBuf::from_iter([&buf, &"stratum.json".parse().unwrap()].iter()).into_boxed_path();

    let p2p_cfg_path =
        PathBuf::from_iter([&buf, &"p2p.json".parse().unwrap()].iter()).into_boxed_path();

    let stratum_config: ProtocolServerConfig<StratumConfig> = read_config(&stratum_cfg_path)?;

    let p2p_config: ProtocolServerConfig<ConfigP2P> = read_config(&p2p_cfg_path)?;

    info!("Stratum config: {:#?}", &stratum_config);
    info!("P2P config: {:#?}", &p2p_config);

    let mut p2p_server: ServerP2P<bitcoincore_rpc::Client> =
        ServerP2P::new(p2p_config, buf.into_boxed_path());

    let mut stratum_server: StratumServer<bitcoincore_rpc::Client> =
        StratumServer::new(stratum_config, p2p_server.protocol.clone());

    let stratum_thread = std::thread::spawn(move || loop {
        stratum_server.process_stratum();
    });

    let p2p_thread = std::thread::spawn(move || loop {
        p2p_server.process_p2p();
    });

    stratum_thread.join().unwrap();
    p2p_thread.join().unwrap();

    Ok(())
}
