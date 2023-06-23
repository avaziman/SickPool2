use display_bytes::display_bytes_string;
use io_arc::IoArc;
use itertools::Itertools;
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
    type Notification;

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
    fn poll_notifications(&self) -> (MutexGuard<Slab<Notifier>>, Self::Request) {
        loop {}
    }
}
pub struct JsonRpcProtocol<UP> {
    pub up: UP,
}

// UNDERLYING PROTOCOL
// this layer is responsibile for keeping the rpc req id, and forwarding the underlying request to the ud protocol
impl<UP, E> Protocol for JsonRpcProtocol<UP>
where
    E: std::fmt::Display + Discriminant,
    UP: Protocol<Request = RpcReqBody, Response = Result<Value, E>, Notification = RpcReqBody>,
{
    type Request = Vec<u8>;
    type Response = Vec<u8>;
    type Config = UP::Config;
    type ClientContext = UP::ClientContext;
    type ProcessingContext = UP::ProcessingContext;
    type Notification = Vec<u8>;

    fn new(conf: Self::Config) -> Self {
        Self { up: UP::new(conf) }
    }

    fn process_request(
        &self,
        req: Self::Request,
        ctx: Arc<Mutex<Self::ClientContext>>,
        ptx: &mut Self::ProcessingContext,
    ) -> Self::Response {
        let mut bytes = serde_json::to_vec(&match Self::parse_request(&req) {
            Ok(rpc_request) => RpcResponse::new(
                rpc_request.id,
                self.up
                    .process_request((rpc_request.method, rpc_request.params), ctx, ptx),
            ),
            Err(e) => {
                warn!(
                    "Failed to parse jsonrpc request: {}, req: {:?}",
                    e,
                    display_bytes_string(&req)
                );
                RpcResponse {
                    id: None,
                    res_or_err: ResultOrErr::Error((0, String::from("Bad JSON RPC request"), None)),
                }
            }
        })
        .unwrap();
        bytes.push('\n' as u8);
        bytes
    }

    // perhaps save the format of given client requests later... or smt
    fn create_client(&self, addr: SocketAddr, notifier: Notifier) -> Option<Self::ClientContext> {
        self.up.create_client(addr, notifier)
    }

    fn poll_notifications(&self) -> (MutexGuard<Slab<Notifier>>, Self::Request) {
        let (notifiers, (method, params)) = self.up.poll_notifications();

        (
            notifiers,
            serde_json::to_vec(&RpcRequest {
                params,
                method,
                id: None,
                jsonrpc: None,
            })
            .unwrap(),
        )
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
    UP: Protocol<Request = RpcReqBody, Response = Result<Value, E>>,
{
    #[doc(hidden)]
    pub fn parse_request(req: &[u8]) -> Result<RpcRequest, serde_json::Error> {
        serde_json::from_slice::<RpcRequest>(req)
    }
}
