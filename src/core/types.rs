use crate::utils::parsers::deserialize_string_of_bytes;
use ethers::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct AdvanceInput {
    pub metadata: Metadata,
    #[serde(deserialize_with = "deserialize_string_of_bytes")]
    pub payload: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InspectInput {
    #[serde(deserialize_with = "deserialize_string_of_bytes")]
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub enum AdvanceInputType {
    Advance(AdvanceInput),
    Inspect(InspectInput),
}
