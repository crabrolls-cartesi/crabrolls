use crate::{types::address::Address, utils::parsers::deserializers::*};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone)]
pub struct Metadata {
    pub input_index: i32,
    pub msg_sender: Address,
    pub block_number: i64,
    pub timestamp: i64,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
#[serde(rename_all = "lowercase", tag = "status")]
pub enum FinishStatus {
    Accept,
    Reject,
}

#[derive(Deserialize, Debug)]
pub struct Advance {
    pub metadata: Metadata,
    #[serde(deserialize_with = "deserialize_string_of_bytes")]
    pub payload: Vec<u8>,
}

#[derive(Deserialize, Debug)]
pub struct Inspect {
    #[serde(deserialize_with = "deserialize_string_of_bytes")]
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub enum Input {
    Advance(Advance),
    Inspect(Inspect),
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum Output {
    Voucher {
        destination: Address,
        #[serde(serialize_with = "serialize_bytes_as_string")]
        payload: Vec<u8>,
    },
    Notice {
        #[serde(serialize_with = "serialize_bytes_as_string")]
        payload: Vec<u8>,
    },
    Report {
        #[serde(serialize_with = "serialize_bytes_as_string")]
        payload: Vec<u8>,
    },
}
