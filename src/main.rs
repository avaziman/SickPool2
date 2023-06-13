#![deny(unsafe_code)]
use log::info;
use log4rs;
use sickpool2lib::p2p::networking::protocol::{ConfigP2P, ProtocolP2P};
use sickpool2lib::stratum;
use sickpool2lib::stratum::protocol::StratumV1ErrorCodes;
use sickpool2lib::stratum::server::StratumServer;
use std::fs;
use std::io::Result;
use std::str::FromStr;

extern crate sickpool2lib;

use sickpool2lib::stratum::config::StratumConfig;

// type StratumV1Json = JsonRpcProtocol<StratumV1<bitcoincore_rpc::Client>, StratumV1ErrorCodes>;

fn main() -> Result<()> {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();

    let stratum_config: StratumConfig = serde_json::from_str(
        fs::read_to_string("config/stratum.json")
            .expect("Missing config file")
            .as_str(),
    )
    .expect("Invalid config");

    info!("Stratum config: {:?}", &stratum_config);

    let socket = std::net::SocketAddr::from_str("127.0.0.1:34254").unwrap();
    let mut stratum_server: StratumServer<bitcoincore_rpc::Client> = StratumServer::new(socket, stratum_config);

    // let socket = std::net::SocketAddr::from_str("127.0.0.1:34255").unwrap();
    // let mut p2p_server: Server<ProtocolP2P<bitcoincore_rpc::bitcoin::block::Header>> = Server::new(
    //     socket,
    //     ConfigP2P {
    //         peer_connections: 2,
    //     },
    // );

    let stratum_main_thread = std::thread::spawn(move || loop {
        stratum_server.process_stratum();
    });

    // loop {
    //     p2p_server.process_requests();
    // }

    stratum_main_thread.join().unwrap();

    // for i in 0..10 {
    //     let thread = std::thread::spawn(|| loop {
    //         server.clone().process_requests();
    //     });
    // }

    // thread.join();

    Ok(())
}
