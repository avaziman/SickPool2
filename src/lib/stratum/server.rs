use std::io::Write;
use std::thread;
use std::{net::SocketAddr, sync::Arc};

use bitcoincore_rpc::bitcoin;
use bitcoincore_rpc::bitcoin::block::Header;
use log::info;

use crate::config::ProtocolServerConfig;
use crate::p2p::networking::protocol::ProtocolP2P;
use crate::protocol::Protocol;
use crate::server::respond;
use crate::{protocol::JsonRpcProtocol, server::Server};

use super::handler::StratumHandler;
use super::{
    config::StratumConfig, job_fetcher::BlockFetcher, protocol::StratumV1ErrorCodes,
    stratum_v1::StratumV1,
};

type SProtocol<T> = JsonRpcProtocol<StratumV1<T>>;

pub struct StratumServer<T: BlockFetcher<BlockT = bitcoin::Block>> {
    protocol: Arc<SProtocol<T>>,
    server: Server<SProtocol<T>>,
}

impl<T> StratumServer<T>
where
    T: BlockFetcher<BlockT = bitcoin::Block> + Send + Sync + 'static,
{
    pub fn new(
        conf: ProtocolServerConfig<StratumConfig>,
        p2p: Arc<ProtocolP2P<T::BlockT>>,
    ) -> Self {
        let job_poll_interval = conf.protocol_config.job_poll_interval;
        let protocol = Arc::new(SProtocol::<T>::new((conf.protocol_config, p2p)));

        // TODO: move to protocol ?
        let protocol_poll_cp = protocol.clone();
        thread::spawn(move || {
            let protocol = protocol_poll_cp;
            loop {
                thread::sleep(job_poll_interval);
                // info!("Polling job...");

                protocol.up.fetch_new_job(&protocol.up.daemon_cli);
            }
        });

        Self {
            protocol: protocol.clone(),
            server: Server::new(conf.server_config, protocol),
        }
    }

    pub fn process_stratum(&mut self) {
        self.server.process_requests();
    }
}

// TODO: make control server
