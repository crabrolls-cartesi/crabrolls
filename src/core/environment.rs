use crate::types::machine::{Advance, FinishStatus, Input, Inspect, Output};
use crate::utils::address_book::AddressBook;
use crate::{types::address::Address, utils::requests::ClientWrapper};
use serde_json::Value;
use std::error::Error;
use std::future::Future;

pub trait Environment {
    fn send_voucher(
        &self,
        destination: Address,
        payload: Vec<u8>,
    ) -> impl Future<Output = Result<i32, Box<dyn Error>>> + Send;

    fn send_notice(
        &self,
        payload: Vec<u8>,
    ) -> impl Future<Output = Result<i32, Box<dyn Error>>> + Send;

    fn send_report(
        &self,
        payload: Vec<u8>,
    ) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;
}

pub struct Rollup {
    client: ClientWrapper,
    pub address_book: AddressBook,
}

impl Rollup {
    pub fn new(url: String) -> Self {
        Self {
            client: ClientWrapper::new(url),
            address_book: AddressBook::default(),
        }
    }
}

impl Environment for Rollup {
    async fn send_voucher(
        &self,
        destination: Address,
        payload: Vec<u8>,
    ) -> Result<i32, Box<dyn Error>> {
        let voucher = Output::Voucher {
            destination,
            payload,
        };
        let response = self.client.post("voucher", &voucher).await?;
        let output: serde_json::Value = self.client.parse_response(response).await?;
        Ok(output["index"].as_i64().unwrap_or(0) as i32)
    }

    async fn send_notice(&self, payload: Vec<u8>) -> Result<i32, Box<dyn Error>> {
        let notice = Output::Notice { payload };
        let response = self.client.post("notice", &notice).await?;
        let output: Value = self.client.parse_response(response).await?;
        Ok(output["index"].as_i64().unwrap_or(0) as i32)
    }

    async fn send_report(&self, payload: Vec<u8>) -> Result<(), Box<dyn Error>> {
        let report = Output::Report { payload };
        self.client.post("report", &report).await?;
        Ok(())
    }
}

impl Rollup {
    pub async fn finish_and_get_next(
        &self,
        status: FinishStatus,
    ) -> Result<Option<Input>, Box<dyn Error>> {
        let response = self.client.post("finish", &status).await?;

        let response_status = response.status();

        if response_status != 200 && response_status != 202 {
            return Err(Box::from("Failed to finish the current state"));
        } else if response_status == 202 {
            return Ok(None);
        }

        let value: Value = self.client.parse_response(response).await?;
        debug!("Received input: {:?}", value);

        let request_type = value["request_type"]
            .as_str()
            .ok_or("Invalid request type")?;
        let data = value["data"].clone();

        match request_type {
            "advance_state" => {
                let advance_input: Advance = serde_json::from_value(data)?;
                Ok(Some(Input::Advance(advance_input)))
            }
            "inspect_state" => {
                let inspect_input: Inspect = serde_json::from_value(data)?;
                Ok(Some(Input::Inspect(inspect_input)))
            }
            _ => Err(Box::from("Invalid request type")),
        }
    }
}
