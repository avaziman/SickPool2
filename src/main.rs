use log4rs;
use sickpool2lib::protocol::JsonRpcProtocol;
use sickpool2lib::stratum::protocol::StratumV1ErrorCodes;
use std::io::Result;
use std::str::FromStr;

extern crate sickpool2lib;

use sickpool2lib::protocol_server::ProtocolServer;
use sickpool2lib::stratum::config::StratumConfig;
use sickpool2lib::stratum::stratum_v1::StratumV1;

fn main() -> Result<()> {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();

    let socket = std::net::SocketAddr::from_str("127.0.0.1:34254").unwrap();
    let mut server: ProtocolServer<
        JsonRpcProtocol<StratumV1<bitcoincore_rpc::Client>, StratumV1ErrorCodes>,
    > = ProtocolServer::new(
        socket,
        StratumConfig {
            stratum_address: socket,
            rpc_url: String::from("127.0.0.1:18443"),
        },
    );

    let thread = std::thread::spawn(move || loop {
        server.process_requests();
    });

    thread.join();

    Ok(())
}
