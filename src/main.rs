use log4rs;
use sickpool2lib::p2p::networking::protocol::{ConfigP2P, ProtocolP2P};
use sickpool2lib::protocol::{JsonRpcProtocol, Protocol};
use sickpool2lib::stratum::protocol::StratumV1ErrorCodes;
use std::io::Result;
use std::str::FromStr;

extern crate sickpool2lib;

use sickpool2lib::protocol_server::ProtocolServer;
use sickpool2lib::stratum::config::StratumConfig;
use sickpool2lib::stratum::stratum_v1::StratumV1;

type StratumV1Json = JsonRpcProtocol<StratumV1<bitcoincore_rpc::Client>, StratumV1ErrorCodes>;

fn main() -> Result<()> {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();

    let socket = std::net::SocketAddr::from_str("127.0.0.1:34254").unwrap();
    let mut stratum_server: ProtocolServer<StratumV1Json> = ProtocolServer::new(
        socket,
        StratumConfig {
            stratum_address: socket,
            rpc_url: String::from("127.0.0.1:18443"),
        },
    );

    let socket = std::net::SocketAddr::from_str("127.0.0.1:34255").unwrap();
    let mut p2p_server: ProtocolServer<ProtocolP2P<bitcoincore_rpc::bitcoin::block::Header>> =
        ProtocolServer::new(socket, ConfigP2P { peer_connections: 2});

    let stratum_main_thread = std::thread::spawn(move || loop {
        stratum_server.process_requests();
    });

    loop {
        p2p_server.process_requests();
    }

    stratum_main_thread.join().unwrap();

    // for i in 0..10 {
    //     let thread = std::thread::spawn(|| loop {
    //         server.clone().process_requests();
    //     });
    // }

    // thread.join();

    Ok(())
}
