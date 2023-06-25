use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::stratum::protocol::Discriminant;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct RpcRequest {
    // pub method: String,
    pub params: Value,
    pub id: Option<u64>,
    pub method: String,
    #[serde(skip_serializing_if="Option::is_none")]
    pub jsonrpc: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct RpcResponse {
    pub id: Option<u64>,
    #[serde(flatten)]
    pub res_or_err: ResultOrErr,
    // pub jsonrpc: String
}
pub type RpcReqBody = (String, Value);

impl std::fmt::Display for RpcResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(ok) => f.write_str(ok.as_str()),
            Err(_) => todo!(),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ResultOrErr {
    Result(Value),
    Error((u32, String, Option<Value>)),
}

impl RpcResponse {
    pub fn new(id: Option<u64>, res: Value) -> RpcResponse {
        RpcResponse {
            id,
            res_or_err: ResultOrErr::Result(res),
        }
    }

    pub fn new_err<E: std::fmt::Display + Discriminant>(id: Option<u64>, e_code: E) -> RpcResponse {
        RpcResponse {
            id,
            res_or_err: ResultOrErr::Error((e_code.discriminant(), e_code.to_string(), None)),
        }
    }
}
