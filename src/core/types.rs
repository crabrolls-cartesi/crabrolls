use crate::utils::parsers::deserializers::*;
use ethers::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct Metadata {
    pub input_index: i32,
    pub msg_sender: Address,
    pub block_number: i64,
    pub timestamp: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase", tag = "status")]
pub enum FinishStatus {
    Accept,
    Reject,
}

#[derive(Deserialize, Debug)]
pub struct AdvanceInput {
    pub metadata: Metadata,
    #[serde(deserialize_with = "deserialize_string_of_bytes")]
    pub payload: Vec<u8>,
}

#[derive(Deserialize, Debug)]
pub struct InspectInput {
    #[serde(deserialize_with = "deserialize_string_of_bytes")]
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub enum AdvanceInputType {
    Advance(AdvanceInput),
    Inspect(InspectInput),
}

#[derive(Serialize, Debug)]
pub struct Voucher {
    pub destination: Address,
    #[serde(serialize_with = "serialize_bytes_as_string")]
    pub payload: Vec<u8>,
}

#[derive(Serialize, Debug)]
pub struct Notice {
    #[serde(serialize_with = "serialize_bytes_as_string")]
    pub payload: Vec<u8>,
}

#[derive(Serialize, Debug)]
pub struct Report {
    #[serde(serialize_with = "serialize_bytes_as_string")]
    pub payload: Vec<u8>,
}
