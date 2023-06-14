#![deny(unsafe_code)]
use log::info;
use log4rs;
use sickpool2lib::config::ProtocolServerConfig;
use sickpool2lib::p2p::networking::protocol::ConfigP2P;
use sickpool2lib::p2p::networking::server::ServerP2P;
use sickpool2lib::stratum::server::StratumServer;
use std::result::Result;
use std::{env, fs};

extern crate sickpool2lib;

use sickpool2lib::stratum::config::StratumConfig;

// type StratumV1Json = JsonRpcProtocol<StratumV1<bitcoincore_rpc::Client>, StratumV1ErrorCodes>;
fn read_config<T: Default + serde::de::DeserializeOwned + serde::Serialize>(
    path: &str,
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
            return Err(format!("Missing config file at: {}, generated default config, modify it and restart to continue.", path));
        }
    };
    match serde_json::from_str(&data) {
        Ok(k) => Ok(k),
        Err(e) => Err(format!("Invalid config at: {}, {}", path, e)),
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

    let stratum_config: ProtocolServerConfig<StratumConfig> =
        read_config(&format!("{}/config/stratum.json", data_dir))?;

    let p2p_config: ProtocolServerConfig<ConfigP2P> =
        read_config(&format!("{}/config/p2p.json", data_dir))?;

    info!("Stratum config: {:#?}", &stratum_config);
    info!("P2P config: {:#?}", &p2p_config);

    let mut stratum_server: StratumServer<bitcoincore_rpc::Client> =
        StratumServer::new(stratum_config);

    let mut p2p_server: ServerP2P<bitcoincore_rpc::Client> = ServerP2P::new(p2p_config, data_dir);

    let stratum_thread = std::thread::spawn(move || loop {
        stratum_server.process_stratum();
    });

    let p2p_thread = std::thread::spawn(move || loop {
        p2p_server.process_p2p();
    });

    // loop {
    //     p2p_server.process_requests();
    // }

    stratum_thread.join().unwrap();
    p2p_thread.join().unwrap();

    // for i in 0..10 {
    //     let thread = std::thread::spawn(|| loop {
    //         server.clone().process_requests();
    //     });
    // }

    // thread.join();

    Ok(())
}
