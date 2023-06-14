use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use bitcoincore_rpc::bitcoin::block::Header;
use log::info;

use super::protocol::{ConfigP2P, ProtocolP2P};
use crate::protocol::Protocol;
use crate::{
    config::ProtocolServerConfig,
    server::Server,
    stratum::{config::StratumConfig, job_fetcher::HeaderFetcher, server::StratumServer},
};

pub struct ServerP2P<T: HeaderFetcher<HeaderT = Header> + Send + Sync + 'static> {
    // stratum: StratumServer<T>,
    protocol: Arc<ProtocolP2P<T::HeaderT>>,
    server: Server<ProtocolP2P<T::HeaderT>>,
}

impl<T: HeaderFetcher<HeaderT = Header> + Send + Sync + 'static> ServerP2P<T> {
    pub fn new(p2pconf: ProtocolServerConfig<ConfigP2P>, data_dir: String) -> Self {
        let protocol = 
            Arc::new(ProtocolP2P::new((p2pconf.protocol_config, data_dir)));
        let server = Server::new(p2pconf.server_config, protocol.clone());

        let mut se = Self {
            // stratum: StratumServer::new(stratum_conf),
            server,
            protocol,
        };

        se.connect();
        se
    }

    pub fn connect(&mut self) {
        let missing =
            self.protocol.conf.peer_connections - self.server.get_connection_count() as u32;

        // info!("Missing connections...");
        for i in self.protocol.peers_to_connect(missing) {
            if /* self.server.is_connected(i) || */ i == self.server.conf.address {
                continue;
            }

            self.server.connect(i);
        }
    }

    pub fn process_p2p(&mut self) {
        self.server.process_requests();

        // TODO timer...
        self.connect();
    }
}
