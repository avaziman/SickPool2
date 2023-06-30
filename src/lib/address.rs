use bitcoin::{address::NetworkUnchecked, Script, ScriptBuf};
use std::{hash::Hash, str::FromStr};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    coins::{
        bitcoin::{Btc, MyBtcAddr},
        coin::Coin,
    },
    p2p::networking::block::{Block, EncodeErrorP2P},
};
pub trait Address:
    'static
    + Eq
    + PartialEq
    + Hash
    + Clone
    + DeserializeOwned
    + Serialize
    + std::fmt::Debug
    + Send
    + Sync
{
    type FromScript;
    type Error: std::fmt::Debug;

    fn from_script(s: &Self::FromScript) -> Result<Self, Self::Error>;
    fn from_string(s: &str) -> Result<Self, Self::Error>;
    fn to_script(&self) -> Self::FromScript;
}

pub struct BtcLikeAddr;
impl Address for MyBtcAddr {
    type FromScript = bitcoin::script::ScriptBuf;
    type Error = bitcoin::address::Error;

    fn from_script(s: &Self::FromScript) -> Result<Self, Self::Error> {
        let inner = bitcoin::Address::from_script(&s, Btc::NETWORK)?;
        Ok(MyBtcAddr(inner))
    }

    fn to_script(&self) -> Self::FromScript {
        self.0.script_pubkey()
    }

    fn from_string(s: &str) -> Result<Self, Self::Error> {
        Ok(MyBtcAddr(
            bitcoin::Address::<NetworkUnchecked>::from_str(s)?
                .require_network(Btc::NETWORK)?,
        ))
    }
}

// pub trait BtcScriptAddr: Address + Sized {
//     fn to_spend_script(&self) -> ScriptBuf;
//     fn from_spend_script(s: &Script) -> Result<Self, EncodeErrorP2P>;
// }

// impl BtcScriptAddr for MyBtcAddr {
//     fn to_spend_script(&self) -> ScriptBuf {
//     }

//     fn from_spend_script(s: &Script) -> Result<Self, EncodeErrorP2P> {

//     }
// }
