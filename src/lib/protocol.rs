use display_bytes::display_bytes_string;
use io_arc::IoArc;

use log::warn;
use mio::net::TcpStream;
use serde_json::Value;
use slab::Slab;
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex, MutexGuard},
};

use crate::{
    server::Notifier,
    sickrpc::{ResultOrErr, RpcReqBody, RpcRequest, RpcResponse},
    stratum::protocol::Discriminant,
};

pub trait Protocol {
    type Request: std::fmt::Debug;
    type Response;
    type Config;
    type ClientContext: std::fmt::Debug + Sync + Send;
    type ProcessingContext;

    fn new(conf: Self::Config) -> Self;
    fn process_request(
        &self,
        req: Self::Request,
        ctx: Arc<Mutex<Self::ClientContext>>,
        ptx: &mut Self::ProcessingContext,
    ) -> Self::Response;

    // perhaps sent connection count along, to possibly reject based on
    fn create_client(&self, addr: SocketAddr, notifier: Notifier) -> Option<Self::ClientContext>;

    fn delete_client(&self, ctx: Arc<Mutex<Self::ClientContext>>);
    fn client_conncted(&self, _stream: IoArc<TcpStream>, _ctx: Arc<Mutex<Self::ClientContext>>) {}

    fn create_ptx(&self) -> Self::ProcessingContext;
}
pub struct JsonRpcProtocol<UP> {
    pub up: UP,
}

// UNDERLYING PROTOCOL
// this layer is responsibile for keeping the rpc req id, and forwarding the underlying request to the ud protocol
impl<UP, E> Protocol for JsonRpcProtocol<UP>
where
    E: std::fmt::Display + Discriminant,
    UP: Protocol<Request = RpcReqBody, Response = Result<(Value, Vec<RpcReqBody>), E>>,
{
    type Request = Vec<u8>;
    type Response = Vec<u8>;
    type Config = UP::Config;
    type ClientContext = UP::ClientContext;
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
        let (res, notifs) = match Self::parse_request(&req) {
            Ok(rpc_request) => {
                match self
                    .up
                    .process_request((rpc_request.method, rpc_request.params), ctx, ptx)
                {
                    Ok((res, notifs)) => (RpcResponse::new(rpc_request.id, res), notifs),
                    Err(e) => (RpcResponse::new_err(rpc_request.id, e), Vec::new()),
                }
            }
            Err(e) => {
                warn!(
                    "Failed to parse jsonrpc request: {}, req: {:?}",
                    e,
                    display_bytes_string(&req)
                );
                (
                    RpcResponse {
                        id: None,
                        res_or_err: ResultOrErr::Error((
                            0,
                            String::from("Bad JSON RPC request"),
                            None,
                        )),
                    },
                    Vec::new(),
                )
            }
        };

        let mut bytes = serde_json::to_vec(&res).unwrap();
        bytes.push('\n' as u8);

        for not in notifs.into_iter() {
            let notification = Self::to_notification(not);
            bytes.append(&mut serde_json::to_vec(&notification).unwrap());
            bytes.push('\n' as u8);
        }

        bytes
    }

    // perhaps save the format of given client requests later... or smt
    fn create_client(&self, addr: SocketAddr, notifier: Notifier) -> Option<Self::ClientContext> {
        self.up.create_client(addr, notifier)
    }

    fn delete_client(&self, ctx: Arc<Mutex<Self::ClientContext>>) {
        self.up.delete_client(ctx)
    }

    fn create_ptx(&self) -> Self::ProcessingContext {
        self.up.create_ptx()
    }
}

impl<UP, E> JsonRpcProtocol<UP>
where
    E: std::fmt::Display + Discriminant,

    UP: Protocol<Request = RpcReqBody, Response = Result<(Value, Vec<RpcReqBody>), E>>,
{
    #[doc(hidden)]
    pub fn parse_request(req: &[u8]) -> Result<RpcRequest, serde_json::Error> {
        serde_json::from_slice::<RpcRequest>(req)
    }

    fn to_notification(req: RpcReqBody) -> RpcRequest {
        let (method, params) = req;

        RpcRequest {
            method,
            params,
            id: None,
            jsonrpc: None,
        }
    }

    pub fn notify(req: RpcReqBody, not: &Notifier) {
        let req = Self::to_notification(req);
        let mut bytes = serde_json::to_vec(&req).unwrap();
        bytes.push('\n' as u8);

        not.notify(&bytes);
    }
}
