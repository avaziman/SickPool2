use std::net::SocketAddr;
use log::warn;
use serde::Deserialize;
use serde_json::{Value};

use crate::server::Server;

#[derive(Deserialize, Debug)]
pub struct RpcRequest {
    pub method: String,
    pub params: Value,
    pub id: Value,
    pub jsonrpc: Option<String>,
}

pub struct RpcServer {
    server: Server,
}

impl RpcServer {
    pub fn new(saddr: SocketAddr) -> RpcServer{
        RpcServer { server: Server::new(saddr) }
    }

    pub fn get_requests(&mut self) -> Vec<RpcRequest> {
        self.server.get_requests().iter().filter_map(|req| self.parse(req)).collect()
    }

    fn parse(&self, req: &String) -> Option<RpcRequest> {
        match serde_json::from_str(&req) {
            Ok(req) => Some(req),
            Err(e) => {
                warn!("Failed to parse rpc request: {}", e);
                None
            }
        }
    }
}