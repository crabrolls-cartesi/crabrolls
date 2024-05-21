use super::context::Context;
use super::types::{AdvanceInput, AdvanceInputType, FinishStatus, InspectInput};
use crate::utils::requests::ClientWrapper;
use ethers::prelude::*;
use ethers::utils::*;
use serde_json::{json, Value};
use std::error::Error;

pub trait Environment {
    fn send_voucher(
        &self,
        ctx: &Context,
        destination: Address,
        payload: Vec<u8>,
    ) -> impl std::future::Future<Output = Result<i32, Box<dyn Error>>> + Send;

    fn send_notice(
        &self,
        ctx: &Context,
        payload: Vec<u8>,
    ) -> impl std::future::Future<Output = Result<i32, Box<dyn Error>>> + Send;

    fn send_report(
        &self,
        ctx: &Context,
        payload: Vec<u8>,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn Error>>> + Send;
}

pub struct Rollup {
    client: ClientWrapper,
}

impl Rollup {
    pub fn new(url: String) -> Self {
        Self {
            client: ClientWrapper::new(url),
        }
    }
}

impl Environment for Rollup {
    async fn send_voucher(
        &self,
        _ctx: &Context,
        destination: Address,
        payload: Vec<u8>,
    ) -> Result<i32, Box<dyn Error>> {
        let request = json!({
            "destination": destination,
            "payload": hex::encode(payload),
        });
        let response = self.client.post("voucher", &request).await?;
        let output: serde_json::Value = self.client.parse_response(response).await?;
        Ok(output["index"].as_i64().unwrap_or(0) as i32)
    }

    async fn send_notice(&self, _ctx: &Context, payload: Vec<u8>) -> Result<i32, Box<dyn Error>> {
        let request = json!({
            "payload": hex::encode(payload),
        });
        let response = self.client.post("notice", &request).await?;
        let output: Value = self.client.parse_response(response).await?;
        Ok(output["index"].as_i64().unwrap_or(0) as i32)
    }

    async fn send_report(&self, _ctx: &Context, payload: Vec<u8>) -> Result<(), Box<dyn Error>> {
        let request = json!({
            "payload": hex::encode(payload),
        });
        self.client.post("report", &request).await?;
        Ok(())
    }
}

impl Rollup {
    pub async fn finish_and_get_next(
        &self,
        _ctx: &Context,
        status: FinishStatus,
    ) -> Result<Option<AdvanceInputType>, Box<dyn Error>> {
        let response = self.client.post("finish", &status).await?;

        let response_status = response.status();

        if !response_status.is_success() {
            return Err(Box::from("Failed to finish the current state"));
        } else if response_status == 202 {
            return Ok(None);
        }

        let value: Value = response.json().await?;
        debug!("Received input: {:?}", value);

        let request_type = value["request_type"]
            .as_str()
            .ok_or("Invalid request type")?;
        let data = value["data"].clone();

        match request_type {
            "advance_state" => {
                let advance_input: AdvanceInput = serde_json::from_value(data)?;
                Ok(Some(AdvanceInputType::Advance(advance_input)))
            }
            "inspect_state" => {
                let inspect_input: InspectInput = serde_json::from_value(data)?;
                Ok(Some(AdvanceInputType::Inspect(inspect_input)))
            }
            _ => Err(Box::from("Invalid request type")),
        }
    }
}
