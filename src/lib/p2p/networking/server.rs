

use std::path::Path;
use std::sync::{Arc};

use bitcoincore_rpc::bitcoin;






use super::config::ConfigP2P;
use super::protocol::{ProtocolP2P};


use crate::protocol::Protocol;

use crate::{
    config::ProtocolServerConfig,
    server::Server,
    stratum::{job_fetcher::BlockFetcher},
};

// can operate without a stratum server
pub struct ServerP2P<Fetcher: BlockFetcher> {
    // stratum: StratumServer<T>,
    pub protocol: Arc<ProtocolP2P<Fetcher::BlockT>>,
    server: Server<ProtocolP2P<Fetcher::BlockT>>,
}

impl<T: BlockFetcher<BlockT = bitcoin::Block> + Send + Sync + 'static> ServerP2P<T> {
    pub fn new(p2pconf: ProtocolServerConfig<ConfigP2P>, data_dir: Box<Path>) -> Self {
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
        };

        se.connect();
        se
    }

    pub fn connect(&mut self) {
        let missing =
            self.protocol.conf.peer_connections - self.server.get_connection_count() as u32;

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
