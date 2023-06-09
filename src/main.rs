use log4rs;
use sickpool2lib::protocol::{JsonRpcProtocol};
use sickpool2lib::stratum::protocol::StratumV1ErrorCodes;
use std::io::Result;
use std::str::FromStr;

extern crate sickpool2lib;

use sickpool2lib::protocol_server::ProtocolServer;
use sickpool2lib::stratum::config::StratumConfig;
use sickpool2lib::stratum::stratum_v1::{StratumV1};

type StratumV1Json = JsonRpcProtocol<StratumV1<bitcoincore_rpc::Client>, StratumV1ErrorCodes>;

fn main() -> Result<()> {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();

    let socket = std::net::SocketAddr::from_str("127.0.0.1:34254").unwrap();
    let mut pserver: ProtocolServer<StratumV1Json> = ProtocolServer::new(
        socket,
        StratumConfig {
            stratum_address: socket,
            rpc_url: String::from("127.0.0.1:18443"),
        },
    );


    
    loop {
        pserver.process_requests();
    }

    // for i in 0..10 {
    //     let thread = std::thread::spawn(|| loop {
    //         server.clone().process_requests();
    //     });
    // }

    // thread.join();

    Ok(())
}
