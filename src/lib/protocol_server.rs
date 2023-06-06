use std::{net::SocketAddr};

use crate::{
    protocol::Protocol,
    server::{Server},
};
use log::{error, info, warn};

pub struct ProtocolServer<P: Protocol> {
    server: Server<P::ClientContext>,
    protocol: P,
}

impl<P: Protocol<Request = String, Response = String>> ProtocolServer<P> {
    pub fn new(addr: SocketAddr, protocol_conf: P::Config) -> Self {
        ProtocolServer {
            server: Server::new(addr),
            protocol: P::new(protocol_conf),
        }
    }

    pub fn process_requests(&mut self) {
        let (requests, new_conns, remove_conns) = self.server.get_requests();
        // first priority to process requests, then new connects and disconnects
        for (req, token, ctx) in requests {
            info!("Received request: {:?}", req);
            let stratum_resp = self.protocol.process_request(req);
            self.server.respond(token, stratum_resp.as_ref());
            info!("Responded: {:?}", stratum_resp);
        }

        // CLEANUP
        // for new_conn in new_conns {
        //     self.clients.insert(StratumClient::new());
        // }

        // for remove_conn in remove_conns {
        //     self.clients.remove(remove_conn.0);
        // }
        // TODO, remove problem
    }

    // fn process_request(
    //     &mut self,
    //     req: SH::Request,
    //     token: Token,
    // ) -> String {
    //     // let client = match self.clients.get_mut(token.0) {
    //     //     Some(client) => client,
    //     //     None => {
    //     //         error!("Missing client with token {}", token.0);
    //     //         return Err(StratumV1ErrorCodes::Unknown);
    //     //     }
    //     // };
    //     self.stratum_handler.process_request(req).to_string()
    // }
}

// struct RequestProcessorContext<T: BlockHeader> {
//     // more efficient to update the job list on each thread every new job
//     // to allow mutating each job in parallel
//     jobs: Vec<Job<T>>,
// }

#[cfg(test)]
pub mod tests {

    use mio::Token;
    use serde_json::{json, Value};

    use crate::protocol::JsonRpcProtocol;
    use crate::stratum::protocol::{self, StratumV1ErrorCodes};
    use crate::stratum::stratum_v1::StratumV1;
    use crate::{
        sickrpc::{self, RpcRequest},
        stratum::protocol::{AuthorizeReqParams, StratumRequestsBtc, SubmitReqParams},
    };

    #[test]
    pub fn submit_parse() {
        let req = r#"{"params": ["slush.miner1", "000000bf", "00000001", "504e86ed", "b2957c02"], "id": 4, "method": "mining.submit"}"#;
        let req = String::from(req);

        // let rpc_result: (String, Token) = rpc_server::parse_req(&req).unwrap();
        let result = JsonRpcProtocol::<StratumV1::<bitcoincore_rpc::Client>, StratumV1ErrorCodes>::parse_request(&req).unwrap();
        // JsonRpcProtocol::parse_req(&req).unwrap();
        // assert_eq!(result id: Some(4), method: String::from("mining.submit"), jsonrpc: None })
        assert_eq!(result.id, Some(4));
        assert_eq!(result.method, String::from("mining.submit"));
        assert_eq!(result.jsonrpc, None);

        let stratum_req = StratumV1::<bitcoincore_rpc::Client>::parse_stratum_req(result.method, result.params).unwrap();

        assert_eq!(
            stratum_req,
            StratumRequestsBtc::Submit(SubmitReqParams {
                worker_name: String::from("slush.miner1"),
                job_id: 0xbf,
                nonce2: 0x00000001,
                time: 0x504e86ed,
                nonce: 0xb2957c02
            })
        );
    }

    // #[test]
    // fn authorize_parse() {
    //     let req =
    //         r#"{"params": ["slush.miner1", "password"], "id": 2, "method": "mining.authorize"}"#;

    //     let req = String::from(req);

    //     // let rpc_result: (String, Token) = rpc_server::parse_req(&req).unwrap();
    //     let result: StratumRequestV1 =
    //         StratumV1::<bitcoincore_rpc::Client>::parse_req(&req).unwrap();
    //     assert_eq!(
    //         result,
    //         StratumRequestV1 {
    //             id: Some(2),
    //             stratum_request: StratumRequestsBtc::Authorize(AuthorizeReqParams {
    //                 username: String::from("slush.miner1"),
    //                 password: String::from("password"),
    //             })
    //         }
    //     );
    // }
}

// TODO: return the rpc correct inheritens and write custom deserializer
