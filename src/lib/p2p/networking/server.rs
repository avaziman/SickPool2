use std::path::Path;
use std::sync::Arc;

use super::config::ConfigP2P;
use super::protocol::ProtocolP2P;

use crate::coins::coin::Coin;
use crate::protocol::Protocol;

use crate::{config::ProtocolServerConfig, server::Server};

// can operate without a stratum server
pub struct ServerP2P<C: Coin> {
    pub protocol: Arc<ProtocolP2P<C>>,
    server: Server<ProtocolP2P<C>>,
}

impl<C: Coin + 'static> ServerP2P<C> {
    pub fn new(p2pconf: ProtocolServerConfig<ConfigP2P<C::BlockT>>) -> Self {
        let protocol = Arc::new(ProtocolP2P::new(p2pconf.protocol_config));
        let server = Server::new(p2pconf.server_config, protocol.clone());

        let mut se = Self { server, protocol };

        se.connect();
        se
    }

    pub fn connect(&mut self) {
        let missing =
            self.protocol.conf.max_peer_connections - self.server.get_connection_count() as u32;

        // info!("Missing connections...");
        for i in self.protocol.peers_to_connect(missing) {
            if i == self.server.conf.address {
                continue;
            }

            let _token = self.server.connect(i);
        }
    }

    pub fn process_p2p(&mut self) {
        self.server.process_requests();

        // }
        // TODO timer...
        // info!("Best shares: {:?}", self.protocol.local_best_shares.read().unwrap());
        self.connect();
    }
}
