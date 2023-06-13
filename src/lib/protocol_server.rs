// use std::{io::Write, net::SocketAddr, sync::Arc, time::Instant};

// use crate::{
//     protocol::Protocol,
//     server::{self, Connection, Server},
// };
// use log::{error, info, warn};
// use mio::Token;
// use threadpool::ThreadPool;

// pub struct ProtocolServer<P: Protocol + Send + Sync> {
//     server: Server<P::ClientContext>,
//     protocol: Arc<P>,
//     tpool: ThreadPool,
// }

// impl<P: Protocol<Request = String, Response = String> + Send + Sync + 'static> ProtocolServer<P> {
//     pub fn new(addr: SocketAddr, protocol_conf: P::Config) -> Self {
//         let mut server = ProtocolServer {
//             server: Server::new(addr),
//             protocol: Arc::new(P::new(protocol_conf)),
//             tpool: threadpool::Builder::new()
//                 .num_threads(8)
//                 .thread_stack_size(8_000_000)
//                 .thread_name("Server protocol processing thread".into())
//                 .build(),
//         };
//         for peer in server.protocol.peers_to_connect() {
//             server.server.connect(peer);
//         }
//         server
//     }

//     pub fn process_requests(&mut self) {
//         let (requests, new_cons, rem_cons) = self.server.read_requests();
//         for (req, writer, ctx) in requests.into_iter() {
//             let protocol: Arc<P> = self.protocol.clone();
//             self.tpool.execute(move || {
//                 let mut ptx = P::ProcessingContext::default();
//                 info!("Received request: {:?}", req);
//                 let now = Instant::now();

//                 let stratum_resp = protocol.process_request(req, ctx, &mut ptx);

//                 let elapsed = now.elapsed().as_micros();
//                 server::respond(writer, stratum_resp.as_ref());

//                 info!("Processed response: {:?}, in {}us", stratum_resp, elapsed);
//             });
//         }

//         if !rem_cons.is_empty() {
//             for peer in self.protocol.peers_to_connect() {
//                 self.server.connect(peer);
//             }
//         }
//     }

//     //     // CLEANUP
//     //     // for new_conn in new_conns {
//     //     //     self.clients.insert(StratumClient::new());
//     //     // }

//     //     // for remove_conn in remove_conns {
//     //     //     self.clients.remove(remove_conn.0);
//     //     // }
//     //     // TODO, remove problem
//     // }
// }

// // struct RequestProcessorContext<T: BlockHeader> {
// //     // more efficient to update the job list on each thread every new job
// //     // to allow mutating each job in parallel
// //     jobs: Vec<Job<T>>,
// // }

// #[cfg(test)]
// pub mod tests {

//     use mio::Token;
//     use serde_json::{json, Value};

//     use crate::protocol::JsonRpcProtocol;
//     use crate::stratum::protocol::{self, StratumV1ErrorCodes};
//     use crate::stratum::stratum_v1::StratumV1;
//     use crate::{
//         sickrpc::{self, RpcRequest},
//         stratum::protocol::{AuthorizeReqParams, StratumRequestsBtc, SubmitReqParams},
//     };

//     #[test]
//     pub fn submit_parse() {
//         let req = r#"{"params": ["slush.miner1", "000000bf", "00000001", "504e86ed", "b2957c02"], "id": 4, "method": "mining.submit"}"#;
//         let req = String::from(req);

//         // let rpc_result: (String, Token) = rpc_server::parse_req(&req).unwrap();
//         let result = JsonRpcProtocol::<StratumV1::<bitcoincore_rpc::Client>, StratumV1ErrorCodes>::parse_request(&req).unwrap();
//         // JsonRpcProtocol::parse_req(&req).unwrap();
//         // assert_eq!(result id: Some(4), method: String::from("mining.submit"), jsonrpc: None })
//         assert_eq!(result.id, Some(4));
//         assert_eq!(result.method, String::from("mining.submit"));
//         assert_eq!(result.jsonrpc, None);

//         let stratum_req =
//             StratumV1::<bitcoincore_rpc::Client>::parse_stratum_req(result.method, result.params)
//                 .unwrap();

//         assert_eq!(
//             stratum_req,
//             StratumRequestsBtc::Submit(SubmitReqParams {
//                 worker_name: String::from("slush.miner1"),
//                 job_id: 0xbf,
//                 nonce2: 0x00000001,
//                 time: 0x504e86ed,
//                 nonce: 0xb2957c02
//             })
//         );
//     }

//     // #[test]
//     // fn authorize_parse() {
//     //     let req =
//     //         r#"{"params": ["slush.miner1", "password"], "id": 2, "method": "mining.authorize"}"#;

//     //     let req = String::from(req);

//     //     // let rpc_result: (String, Token) = rpc_server::parse_req(&req).unwrap();
//     //     let result: StratumRequestV1 =
//     //         StratumV1::<bitcoincore_rpc::Client>::parse_req(&req).unwrap();
//     //     assert_eq!(
//     //         result,
//     //         StratumRequestV1 {
//     //             id: Some(2),
//     //             stratum_request: StratumRequestsBtc::Authorize(AuthorizeReqParams {
//     //                 username: String::from("slush.miner1"),
//     //                 password: String::from("password"),
//     //             })
//     //         }
//     //     );
//     // }
// }

// // TODO: return the rpc correct inheritens and write custom deserializer
