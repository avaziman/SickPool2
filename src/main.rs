use std::io::Result;
use std::str::FromStr;
use log::info;
use log4rs;

mod p2p;

pub mod rpc_server;
pub mod server;

use rpc_server::RpcServer;

fn main() -> Result<()> {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();

    let socket = std::net::SocketAddr::from_str("127.0.0.1:34254").unwrap();
    let mut server = RpcServer::new(socket);
    info!("Started server.");

    loop {
        let requests = server.get_requests();
        info!("Requests: {:?}", requests);
    }

    Ok(())
}