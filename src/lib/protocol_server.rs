#[cfg(test)]
pub mod tests {


    use crate::protocol::JsonRpcProtocol;
    use crate::stratum::stratum_v1::AuthorizeReqParams;
    use crate::stratum::stratum_v1::{StratumRequestsBtc, SubmitReqParams};
    use crate::stratum::stratum_v1::StratumV1;

    #[test]
    pub fn submit_parse() {
        let req = r#"{"params": ["slush.miner1", "000000bf", "00000001", "504e86ed", "b2957c02"], "id": 4, "method": "mining.submit"}"#;
        let req = String::from(req);

        // let rpc_result: (String, Token) = rpc_server::parse_req(&req).unwrap();
        let result = JsonRpcProtocol::<StratumV1>::parse_request(&req.as_bytes()).unwrap();
        // JsonRpcProtocol::parse_req(&req).unwrap();
        // assert_eq!(result id: Some(4), method: String::from("mining.submit"), jsonrpc: None })
        assert_eq!(result.id, Some(4));
        assert_eq!(result.method, String::from("mining.submit"));
        assert_eq!(result.jsonrpc, None);

        let stratum_req =
            StratumV1::parse_stratum_req(result.method, result.params)
                .unwrap();

        assert_eq!(
            stratum_req,
            StratumRequestsBtc::Submit(SubmitReqParams {
                worker_name: String::from("slush.miner1"),
                job_id: 0xbf,
                extranonce2: 0x00000001,
                time: 0x504e86ed,
                nonce: 0xb2957c02
            })
        );
    }

    #[test]
    pub fn authorize_parse() {
        let req =
            r#"{"params": ["slush.miner1", "password"], "id": 2, "method": "mining.authorize"}"#;
        let req = String::from(req);

        // let rpc_result: (String, Token) = rpc_server::parse_req(&req).unwrap();
        let result = JsonRpcProtocol::<StratumV1>::parse_request(&req.as_bytes()).unwrap();

        assert_eq!(result.id, Some(2));
        assert_eq!(result.method, String::from("mining.authorize"));
        assert_eq!(result.jsonrpc, None);

        let stratum_req =
            StratumV1::parse_stratum_req(result.method, result.params)
                .unwrap();

        assert_eq!(
            stratum_req,
            StratumRequestsBtc::Authorize(AuthorizeReqParams {
                username: String::from("slush.miner1"),
                password: String::from("password")
            })
        );
    }

}

// TODO: return the rpc correct inheritens and write custom deserializer
