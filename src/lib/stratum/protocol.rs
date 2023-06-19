use std::fmt;

use serde::{
    Deserialize, Serialize,
};
use serde_hex::*;
use serde_tuple::Deserialize_tuple;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum StratumRequestsBtc {
    Submit(SubmitReqParams),
    Subscribe,
    Authorize(AuthorizeReqParams),
}

#[derive(Serialize, Deserialize_tuple, PartialEq, Debug)]
pub struct AuthorizeReqParams {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize_tuple, PartialEq, Debug)]
pub struct SubmitReqParams {
    pub worker_name: String,
    #[serde(with = "SerHex::<Strict>")]
    pub job_id: u32,
    #[serde(with = "SerHex::<Strict>")]
    pub nonce2: u32,
    #[serde(with = "SerHex::<Strict>")]
    pub time: u32,
    #[serde(with = "SerHex::<Strict>")]
    pub nonce: u32,
}

#[repr(u32)]
#[derive(Debug)]
pub enum StratumV1ErrorCodes {
    Unknown(String) = 20,
    JobNotFound = 21,
    DuplicateShare = 22,
    LowDifficultyShare = 23,
    UnauthorizedWorker = 24,
    NotSubscribed = 25,
}

pub trait Discriminant {
    fn discriminant(&self) -> u32;
}

impl Discriminant for StratumV1ErrorCodes {
    fn discriminant(&self) -> u32 {
        unsafe { *(self as *const Self as *const u32) }
    }
}

impl fmt::Display for StratumV1ErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StratumV1ErrorCodes::Unknown(reason) => write!(f, "{}", reason),
            StratumV1ErrorCodes::JobNotFound => write!(f, "Job not found"),
            StratumV1ErrorCodes::DuplicateShare => write!(f, "Duplicate share"),
            StratumV1ErrorCodes::LowDifficultyShare => write!(f, "Low difficulty share"),
            StratumV1ErrorCodes::UnauthorizedWorker => write!(f, "Unauthorized worker"),
            StratumV1ErrorCodes::NotSubscribed => write!(f, "Client not subscribed"),
        }
    }
}