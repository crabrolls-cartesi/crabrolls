use ethers::{prelude::*, utils::hex};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::address;

#[derive(Serialize, Deserialize, Debug)]
pub struct Metadata {
    pub input_index: i32,
    pub msg_sender: Address,
    pub block_number: i64,
    pub block_timestamp: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum FinishStatus {
    Accept,
    Reject,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AdvanceInput {
    pub metadata: Metadata,
    pub payload: Vec<u8>,
}

impl AdvanceInput {
    pub fn parse(data: Value) -> Result<Self, String> {
        let data = data["data"].as_object().ok_or("Invalid data")?;
        let advance_request = &data["metadata"];
        let payload = data["payload"].as_str().ok_or("Invalid payload")?;
        let msg_sender = advance_request["msg_sender"]
            .as_str()
            .ok_or("Invalid msg_sender")?;
        let block_number = advance_request["block_number"]
            .as_i64()
            .ok_or("Invalid block_number")?;
        let timestamp = advance_request["timestamp"]
            .as_i64()
            .ok_or("Invalid timestamp")?;
        let metadata = Metadata {
            input_index: advance_request["input_index"]
                .as_i64()
                .ok_or("Invalid input_index")? as i32,
            msg_sender: address!(msg_sender),
            block_number,
            block_timestamp: timestamp,
        };
        let payload = hex::decode(payload).map_err(|_| "Failed to decode payload")?;
        Ok(AdvanceInput { metadata, payload })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InspectInput {
    pub payload: Vec<u8>,
}

impl InspectInput {
    pub fn parse(data: Value) -> Result<Self, String> {
        let data = data["data"].as_object().ok_or("Invalid data")?;
        let payload = data["payload"]
            .as_str()
            .ok_or("Invalid payload")?
            .to_owned();
        let payload = hex::decode(payload).map_err(|_| "Failed to decode payload")?;
        Ok(InspectInput { payload })
    }
}

#[derive(Debug)]
pub enum AdvanceInputType {
    Advance(AdvanceInput),
    Inspect(InspectInput),
}
