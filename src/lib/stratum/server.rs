use std::io::Write;
use std::thread;
use std::{net::SocketAddr, sync::Arc};

use bitcoincore_rpc::bitcoin::block::Header;
use log::info;

use crate::config::ProtocolServerConfig;
use crate::protocol::Protocol;
use crate::server::respond;
use crate::{protocol::JsonRpcProtocol, server::Server};

use super::{
    config::StratumConfig, job_fetcher::HeaderFetcher, protocol::StratumV1ErrorCodes,
    stratum_v1::StratumV1,
};

type SProtocol<T> = JsonRpcProtocol<StratumV1<T>, StratumV1ErrorCodes>;

pub struct StratumServer<T: HeaderFetcher<HeaderT = Header> + Send + Sync> {
    protocol: Arc<SProtocol<T>>,
    server: Server<SProtocol<T>>,
}

impl<T: HeaderFetcher<HeaderT = Header> + Send + Sync + 'static> StratumServer<T> {
    pub fn new(conf: ProtocolServerConfig<StratumConfig>) -> Self {
        let job_poll_interval = conf.protocol_config.job_poll_interval;
        let protocol = Arc::new(SProtocol::<T>::new(conf.protocol_config));

        let protocol_poll_cp = protocol.clone();
        thread::spawn(move || {
            let protocol = protocol_poll_cp;
            loop {
                thread::sleep(job_poll_interval);
                // info!("Polling job...");

                if protocol.up.fetch_new_job(&protocol.up.daemon_cli) {
                    info!("New job!");
                    let lock = protocol.up.subscribed_clients.lock().unwrap();
                    for (token, stream) in &*lock {
                        respond(stream.clone(), "NEW JOB");
                    }
                }
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
