use std::sync::Arc;
use std::thread;



use crate::config::ProtocolServerConfig;
use crate::p2p::networking::protocol::ProtocolP2P;
use crate::protocol::Protocol;

use crate::{server::Server};

use super::protocol::StratumProtocol;
use super::{config::StratumConfig};

pub struct StratumServer<T: StratumProtocol> {
    server: Server<T>,
}

impl<T> StratumServer<T>
where
    T: StratumProtocol + Send + Sync + 'static + Protocol<Request = Vec<u8>, Response = Vec<u8>>,
{
    pub fn new(conf: ProtocolServerConfig<StratumConfig>, p2p: Arc<ProtocolP2P<T::Coin>>) -> Self {
        let job_poll_interval = conf.protocol_config.job_poll_interval;
        let protocol = Arc::new(T::new((conf.protocol_config, p2p)));

        let protocol_poll_cp = protocol.clone();
        thread::spawn(move || {
            let protocol = protocol_poll_cp;
            loop {
                protocol.fetch_new_job();
                // info!("Polling job...");
                
                thread::sleep(job_poll_interval);
            }
        });

        Self {
            server: Server::new(conf.server_config, protocol),
        }
    }

    pub fn process_stratum(&mut self) {
        self.server.process_requests();
    }
}

// TODO: make control server
