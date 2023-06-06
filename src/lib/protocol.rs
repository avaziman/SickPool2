use serde_json::Value;

use crate::{
    sickrpc::{ResultOrErr, RpcReqBody, RpcRequest, RpcResponse},
    stratum::protocol::Discriminant,
};

pub trait Protocol {
    type Request: std::fmt::Debug;
    type Response;
    type Config;
    type ClientContext: Default + std::fmt::Debug;

    fn new(conf: Self::Config) -> Self;
    fn process_request(&mut self, req: Self::Request) -> Self::Response;
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

    fn new(conf: Self::Config) -> Self {
        Self { up: UP::new(conf) }
    }

    fn process_request(&mut self, req: Self::Request) -> Self::Response {
        serde_json::to_string(&match Self::parse_request(&req) {
            Ok(rpc_request) => RpcResponse::new(
                rpc_request.id,
                self.up
                    .process_request((rpc_request.method, rpc_request.params)),
            ),
            Err(e) => {
                eprintln!("Failed to parse request: {}", e);
                RpcResponse {
                    id: None,
                    res_or_err: ResultOrErr::Error((0, String::from("Bad JSON RPC request"), None)),
                }
            }
        })
        .unwrap() + "\n"
    }
}

impl<UP, E> JsonRpcProtocol<UP, E>
where
    UP: Protocol<Request = RpcReqBody, Response = Result<Value, E>>,
    E: std::fmt::Display + Discriminant,
{
    #[doc(hidden)]
    pub/* (crate) */ fn parse_request(req: &String) -> Result<RpcRequest, serde_json::Error>{
        serde_json::from_str::<RpcRequest>(req)
    }
}