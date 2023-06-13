use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use log::warn;
use serde_json::Value;

use crate::{
    sickrpc::{ResultOrErr, RpcReqBody, RpcRequest, RpcResponse},
    stratum::protocol::Discriminant,
};

pub trait Protocol {
    type Request: std::fmt::Debug;
    type Response;
    type Config;
    type ClientContext: Default + std::fmt::Debug + Sync + Send;
    type ProcessingContext: Default;

    fn new(conf: Self::Config) -> Self;
    fn process_request(
        &self,
        req: Self::Request,
        ctx: Arc<Mutex<Self::ClientContext>>,
        ptx: &mut Self::ProcessingContext,
    ) -> Self::Response;

    // tells the server who to connect to at bootstrap
    // relevant for p2p protocol only for now
    fn peers_to_connect(&self) -> Vec<SocketAddr> {
        Vec::new()
    }
}
pub struct JsonRpcProtocol<UP, E>
where
    UP: Protocol<Request = RpcReqBody, Response = Result<Value, E>>,
    E: std::fmt::Display + Discriminant,
{
    up: UP,
}

// UNDERLYING PROTOCOL
// this layer is responsibile for keeping the rpc req id, and forwarding the underlying request to the ud protocol
impl<UP, E> Protocol for JsonRpcProtocol<UP, E>
where
    UP: Protocol<Request = RpcReqBody, Response = Result<Value, E>>,
    E: std::fmt::Display + Discriminant,
{
    type Request = String;
    // type Response = RpcResponse;
    type Response = String;
    type Config = UP::Config;
    type ClientContext = UP::ClientContext;
    // cloned per processing thread
    type ProcessingContext = UP::ProcessingContext;

    fn new(conf: Self::Config) -> Self {
        Self { up: UP::new(conf) }
    }

    fn process_request(
        &self,
        req: Self::Request,
        ctx: Arc<Mutex<Self::ClientContext>>,
        ptx: &mut Self::ProcessingContext,
    ) -> Self::Response {
        serde_json::to_string(&match Self::parse_request(&req) {
            Ok(rpc_request) => RpcResponse::new(
                rpc_request.id,
                self.up
                    .process_request((rpc_request.method, rpc_request.params), ctx, ptx),
            ),
            Err(e) => {
                warn!("Failed to parse request: {}", e);
                RpcResponse {
                    id: None,
                    res_or_err: ResultOrErr::Error((0, String::from("Bad JSON RPC request"), None)),
                }
            }
        })
        .unwrap()
            + "\n"
    }
}

impl<UP, E> JsonRpcProtocol<UP, E>
where
    UP: Protocol<Request = RpcReqBody, Response = Result<Value, E>>,
    E: std::fmt::Display + Discriminant,
{
    #[doc(hidden)]
    pub fn parse_request(req: &String) -> Result<RpcRequest, serde_json::Error> {
        serde_json::from_str::<RpcRequest>(req)
    }
}
