use super::contracts::ether::EtherWallet;
use crate::types::machine::{Advance, FinishStatus, Input, Inspect, Output};
use crate::utils::address_book::AddressBook;
use crate::{types::address::Address, utils::requests::ClientWrapper};
use async_std::sync::RwLock;
use ethabi::Uint;
use serde_json::Value;
use std::error::Error;
use std::future::Future;
use std::sync::Arc;

pub trait Environment {
	fn send_voucher(
		&self,
		destination: Address,
		payload: impl AsRef<[u8]> + Send,
	) -> impl Future<Output = Result<i32, Box<dyn Error>>> + Send;

	fn send_notice(&self, payload: impl AsRef<[u8]> + Send)
		-> impl Future<Output = Result<i32, Box<dyn Error>>> + Send;

	fn send_report(&self, payload: impl AsRef<[u8]> + Send) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;

	fn ether_addresses(&self) -> impl Future<Output = Vec<Address>> + Send;
	fn ether_withdraw(&self, address: Address, value: Uint) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;
	fn ether_transfer(
		&self,
		source: Address,
		destination: Address,
		value: Uint,
	) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;
	fn ether_balance(&self, address: Address) -> impl Future<Output = Uint> + Send;
}

pub struct Rollup {
	client: ClientWrapper,
	app_address: Option<Address>,

	pub address_book: AddressBook,
	pub ether_wallet: Arc<RwLock<EtherWallet>>,
}

impl Rollup {
	pub fn new(url: String) -> Self {
		Self {
			client: ClientWrapper::new(url),
			app_address: None,
			address_book: AddressBook::default(),
			ether_wallet: Arc::new(RwLock::new(EtherWallet::new())),
		}
	}

	pub fn set_app_address(&mut self, address: Address) {
		debug!("Setting app address to: {}", address);
		self.app_address = Some(address);
	}
}

impl Environment for Rollup {
	async fn send_voucher(
		&self,
		destination: Address,
		payload: impl AsRef<[u8]> + Send,
	) -> Result<i32, Box<dyn Error>> {
		let voucher = Output::Voucher {
			destination,
			payload: payload.as_ref().to_vec(),
		};
		let response = self.client.post("voucher", &voucher).await?;
		let output: serde_json::Value = self.client.parse_response(response).await?;
		Ok(output["index"].as_i64().unwrap_or(0) as i32)
	}

	async fn send_notice(&self, payload: impl AsRef<[u8]> + Send) -> Result<i32, Box<dyn Error>> {
		let notice = Output::Notice {
			payload: payload.as_ref().to_vec(),
		};
		let response = self.client.post("notice", &notice).await?;
		let output: Value = self.client.parse_response(response).await?;
		Ok(output["index"].as_i64().unwrap_or(0) as i32)
	}

	async fn send_report(&self, payload: impl AsRef<[u8]> + Send) -> Result<(), Box<dyn Error>> {
		let report = Output::Report {
			payload: payload.as_ref().to_vec(),
		};
		self.client.post("report", &report).await?;
		Ok(())
	}

	async fn ether_addresses(&self) -> Vec<Address> {
		self.ether_wallet.read().await.addresses()
	}

	async fn ether_withdraw(&self, address: Address, value: Uint) -> Result<(), Box<dyn Error>> {
		if self.app_address.is_none() {
			return Err(Box::from("App address is not set"));
		}

		let mut ether_wallet = self.ether_wallet.write().await;
		let payload = ether_wallet.withdraw(address, value)?;

		self.send_voucher(self.app_address.expect("App address is not set"), payload)
			.await?;

		Ok(())
	}

	async fn ether_transfer(&self, source: Address, destination: Address, value: Uint) -> Result<(), Box<dyn Error>> {
		if self.app_address.is_none() {
			return Err(Box::from("App address is not set"));
		}

		let mut ether_wallet = self.ether_wallet.write().await;
		ether_wallet.transfer(source, destination, value)?;

		Ok(())
	}

	async fn ether_balance(&self, address: Address) -> Uint {
		self.ether_wallet.read().await.balance_of(address)
	}
}

impl Rollup {
	pub async fn finish_and_get_next(&self, status: FinishStatus) -> Result<Option<Input>, Box<dyn Error>> {
		let response = self.client.post("finish", &status).await?;

		let response_status = response.status();

		if response_status != 200 && response_status != 202 {
			return Err(Box::from("Failed to finish the current state"));
		} else if response_status == 202 {
			return Ok(None);
		}

		let value: Value = self.client.parse_response(response).await?;
		debug!("Received input: {:?}", value);

		let request_type = value["request_type"].as_str().ok_or("Invalid request type")?;
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
