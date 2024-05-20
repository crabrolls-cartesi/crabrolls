use super::context::Context;
use super::types::{AdvanceInput, AdvanceInputType, FinishStatus, InspectInput};
use crate::utils::requests::{parse_response, send_post};
use __serde_json::json;
use ethers::prelude::*;
use ethers::utils::*;
use reqwest::Client;
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
    url: String,
    client: Client,
}

impl Rollup {
    pub fn new(url: String) -> Self {
        Self {
            url,
            client: Client::new(),
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
        let request = serde_json::json!({
            "destination": destination,
            "payload": hex::encode(payload),
        });
        let response = send_post(&self.client, &self.url, "voucher", &request).await?;
        let output: serde_json::Value = parse_response(response).await?;
        Ok(output["index"].as_i64().unwrap_or(0) as i32)
    }

    async fn send_notice(&self, _ctx: &Context, payload: Vec<u8>) -> Result<i32, Box<dyn Error>> {
        let request = serde_json::json!({
            "payload": hex::encode(payload),
        });
        let response = send_post(&self.client, &self.url, "notice", &request).await?;
        let output: serde_json::Value = parse_response(response).await?;
        Ok(output["index"].as_i64().unwrap_or(0) as i32)
    }

    async fn send_report(&self, _ctx: &Context, payload: Vec<u8>) -> Result<(), Box<dyn Error>> {
        let request = serde_json::json!({
            "payload": hex::encode(payload),
        });
        send_post(&self.client, &self.url, "report", &request).await?;
        Ok(())
    }
}

impl Rollup {
    pub async fn finish_and_get_next(
        &self,
        _ctx: &Context,
        status: FinishStatus,
    ) -> Result<Option<AdvanceInputType>, Box<dyn Error>> {
        let response = send_post(
            &self.client,
            &self.url,
            "finish",
            &json!({"status": status}),
        )
        .await?;
        let response_status = response.status();

        if !response_status.is_success() {
            return Err(Box::from("Failed to finish"));
        } else if response_status == 202 {
            return Ok(None);
        }

        let value: serde_json::Value = response.json().await?;
        debug!("Received input: {:?}", value);

        match value["request_type"]
            .as_str()
            .ok_or("Invalid request type")?
        {
            "advance_state" => {
                let advance_input = AdvanceInput::parse(value)?;
                Ok(Some(AdvanceInputType::Advance(advance_input)))
            }
            "inspect_state" => {
                let inspect_input = InspectInput::parse(value)?;
                Ok(Some(AdvanceInputType::Inspect(inspect_input)))
            }
            _ => Err(Box::from("Invalid request type")),
        }
    }
}
