use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use bitcoincore_rpc::bitcoin::block::Header;
use io_arc::IoArc;
use log::info;
use mio::net::TcpStream;

use super::protocol::{ConfigP2P, ProtocolP2P};
use crate::p2p::networking::hard_config::CURRENT_VERSION;
use crate::p2p::networking::protocol::{Hello, Messages};
use crate::protocol::Protocol;
use crate::{
    config::ProtocolServerConfig,
    server::Server,
    stratum::{config::StratumConfig, job_fetcher::HeaderFetcher, server::StratumServer},
};

// can operate without a stratum server
pub struct ServerP2P<T: HeaderFetcher<HeaderT = Header> + Send + Sync + 'static> {
    // stratum: StratumServer<T>,
    protocol: Arc<ProtocolP2P<T::HeaderT>>,
    server: Server<ProtocolP2P<T::HeaderT>>,
    pending_connections: Vec<IoArc<TcpStream>>,
}

impl<T: HeaderFetcher<HeaderT = Header> + Send + Sync + 'static> ServerP2P<T> {
    pub fn new(p2pconf: ProtocolServerConfig<ConfigP2P>, data_dir: String) -> Self {
        let protocol = Arc::new(ProtocolP2P::new((
            p2pconf.protocol_config,
            data_dir,
            p2pconf.server_config.address.port(),
        )));
        let server = Server::new(p2pconf.server_config, protocol.clone());

        let mut se = Self {
            // stratum: StratumServer::new(stratum_conf),
            server,
            protocol,
            pending_connections: Vec::new(),
        };

        se.connect();
        se
    }

    pub fn connect(&mut self) {
        let missing =
            self.protocol.conf.peer_connections - self.server.get_connection_count() as u32;

        // info!("Missing connections...");
        for i in self.protocol.peers_to_connect(missing) {
            if
            /* self.server.is_connected(i) || */
            i == self.server.conf.address {
                continue;
            }

            let token = self.server.connect(i);
            if let Some(token) = token {
                self.pending_connections.push(token);
            }
        }
    }

    pub fn process_p2p(&mut self) {
        self.server.process_requests();

        // for i in &self.pending_connections {
        //     println!("Pending connection to {:?}", i);

        // }
        // TODO timer...
        self.connect();
    }
}
