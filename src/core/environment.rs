use super::contracts::erc1155::{ERC1155Environment, ERC1155Wallet, IntoIdsAmountsIter};
use super::contracts::erc20::{ERC20Environment, ERC20Wallet};
use super::contracts::erc721::{ERC721Environment, ERC721Wallet};
use super::contracts::ether::{EtherEnvironment, EtherWallet};
use crate::types::machine::{Advance, FinishStatus, Input, Inspect, Output};
use crate::utils::address_book::AddressBook;
use crate::utils::requests::ClientWrapper;
use async_std::sync::RwLock;
use ethabi::{Address, Uint};
use serde_json::Value;
use std::error::Error;
use std::future::Future;
use std::sync::Arc;

pub trait Environment:
	EtherEnvironment + ERC20Environment + ERC721Environment + ERC1155Environment + RollupInternalEnvironment
{
	fn send_voucher(
		&self,
		destination: Address,
		payload: impl AsRef<[u8]> + Send,
	) -> impl Future<Output = Result<i32, Box<dyn Error>>> + Send;

	fn send_notice(&self, payload: impl AsRef<[u8]> + Send)
		-> impl Future<Output = Result<i32, Box<dyn Error>>> + Send;

	fn send_report(&self, payload: impl AsRef<[u8]> + Send) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;
}

pub trait RollupInternalEnvironment {
	fn get_address_book(&self) -> AddressBook;
	fn get_ether_wallet(&self) -> Arc<RwLock<EtherWallet>>;
	fn get_erc20_wallet(&self) -> Arc<RwLock<ERC20Wallet>>;
	fn get_erc721_wallet(&self) -> Arc<RwLock<ERC721Wallet>>;
	fn get_erc1155_wallet(&self) -> Arc<RwLock<ERC1155Wallet>>;
}

pub struct Rollup {
	client: ClientWrapper,
	app_address: Arc<RwLock<Option<Address>>>,

	address_book: AddressBook,
	ether_wallet: Arc<RwLock<EtherWallet>>,
	erc20_wallet: Arc<RwLock<ERC20Wallet>>,
	erc721_wallet: Arc<RwLock<ERC721Wallet>>,
	erc1155_wallet: Arc<RwLock<ERC1155Wallet>>,
}

impl Rollup {
	pub fn new(url: &'static str, address_book: AddressBook) -> Self {
		Self {
			client: ClientWrapper::new(url.into()),
			app_address: Arc::new(RwLock::new(None)),
			address_book: address_book,
			ether_wallet: Arc::new(RwLock::new(EtherWallet::new())),
			erc20_wallet: Arc::new(RwLock::new(ERC20Wallet::new())),
			erc721_wallet: Arc::new(RwLock::new(ERC721Wallet::new())),
			erc1155_wallet: Arc::new(RwLock::new(ERC1155Wallet::new())),
		}
	}

	pub async fn set_app_address(&self, address: Address) {
		debug!("Setting app address to: {}", address);
		self.app_address.write().await.replace(address);
	}
}

impl RollupInternalEnvironment for Rollup {
	fn get_address_book(&self) -> AddressBook {
		self.address_book.clone()
	}

	fn get_ether_wallet(&self) -> Arc<RwLock<EtherWallet>> {
		self.ether_wallet.clone()
	}

	fn get_erc20_wallet(&self) -> Arc<RwLock<ERC20Wallet>> {
		self.erc20_wallet.clone()
	}

	fn get_erc721_wallet(&self) -> Arc<RwLock<ERC721Wallet>> {
		self.erc721_wallet.clone()
	}

	fn get_erc1155_wallet(&self) -> Arc<RwLock<ERC1155Wallet>> {
		self.erc1155_wallet.clone()
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
}

impl EtherEnvironment for Rollup {
	async fn ether_addresses(&self) -> Vec<Address> {
		self.ether_wallet.read().await.addresses()
	}

	async fn ether_withdraw(&self, address: Address, value: Uint) -> Result<(), Box<dyn Error>> {
		let app_address = self.app_address.read().await;
		if app_address.is_none() {
			return Err(Box::from("App address is not set"));
		}

		let mut ether_wallet = self.ether_wallet.write().await;
		let payload = ether_wallet.withdraw(address, value)?;

		self.send_voucher(app_address.expect("App address is not set"), payload)
			.await?;

		Ok(())
	}

	async fn ether_transfer(&self, source: Address, destination: Address, value: Uint) -> Result<(), Box<dyn Error>> {
		let mut ether_wallet = self.ether_wallet.write().await;
		ether_wallet.transfer(source, destination, value)?;

		Ok(())
	}

	async fn ether_balance(&self, address: Address) -> Uint {
		self.ether_wallet.read().await.balance_of(address)
	}
}

impl ERC20Environment for Rollup {
	async fn erc20_addresses(&self) -> Vec<Address> {
		self.erc20_wallet.read().await.addresses()
	}

	async fn erc20_withdraw(
		&self,
		wallet_address: Address,
		token_address: Address,
		value: Uint,
	) -> Result<(), Box<dyn Error>> {
		let mut erc20_wallet = self.erc20_wallet.write().await;
		let payload = erc20_wallet.withdraw(wallet_address, token_address, value)?;

		self.send_voucher(token_address, payload).await?;

		Ok(())
	}

	async fn erc20_transfer(
		&self,
		src_wallet: Address,
		dst_wallet: Address,
		token_address: Address,
		value: Uint,
	) -> Result<(), Box<dyn Error>> {
		let mut erc20_wallet = self.erc20_wallet.write().await;
		erc20_wallet.transfer(src_wallet, dst_wallet, token_address, value)?;

		Ok(())
	}

	async fn erc20_balance(&self, wallet_address: Address, token_address: Address) -> Uint {
		self.erc20_wallet.read().await.balance_of(wallet_address, token_address)
	}
}

impl ERC721Environment for Rollup {
	async fn erc721_addresses(&self) -> Vec<Address> {
		self.erc721_wallet.read().await.addresses()
	}

	async fn erc721_withdraw(
		&self,
		wallet_address: Address,
		token_address: Address,
		token_id: Uint,
	) -> Result<(), Box<dyn Error>> {
		let app_address = self.app_address.read().await;
		if app_address.is_none() {
			return Err(Box::from("App address is not set"));
		}

		let mut erc721_wallet = self.erc721_wallet.write().await;
		let payload = erc721_wallet.withdraw(
			app_address.expect("App address is not set"),
			wallet_address,
			token_address,
			token_id,
		)?;

		self.send_voucher(token_address, payload).await?;

		Ok(())
	}

	async fn erc721_transfer(
		&self,
		src_wallet: Address,
		dst_wallet: Address,
		token_address: Address,
		token_id: Uint,
	) -> Result<(), Box<dyn Error>> {
		let mut erc721_wallet = self.erc721_wallet.write().await;
		erc721_wallet.transfer(src_wallet, dst_wallet, token_address, token_id)?;

		Ok(())
	}

	async fn erc721_owner_of(&self, token_address: Address, token_id: Uint) -> Option<Address> {
		self.erc721_wallet.read().await.owner_of(token_address, token_id)
	}
}

impl ERC1155Environment for Rollup {
	async fn erc1155_addresses(&self) -> Vec<Address> {
		self.erc1155_wallet.read().await.addresses()
	}

	async fn erc1155_withdraw<I>(
		&self,
		wallet_address: Address,
		token_address: Address,
		withdrawals: I,
		data: Option<Vec<u8>>,
	) -> Result<(), Box<dyn Error>>
	where
		I: IntoIdsAmountsIter,
	{
		let app_address = self.app_address.read().await;
		if app_address.is_none() {
			return Err(Box::from("App address is not set"));
		}

		let mut erc1155_wallet = self.erc1155_wallet.write().await;
		let payload = erc1155_wallet.withdraw(
			app_address.expect("App address is not set"),
			wallet_address,
			token_address,
			withdrawals,
			data,
		)?;

		self.send_voucher(token_address, payload).await?;

		Ok(())
	}

	async fn erc1155_transfer<I>(
		&self,
		src_wallet: Address,
		dst_wallet: Address,
		token_address: Address,
		transfers: I,
	) -> Result<(), Box<dyn Error>>
	where
		I: IntoIdsAmountsIter,
	{
		let mut erc1155_wallet = self.erc1155_wallet.write().await;
		erc1155_wallet.transfer(src_wallet, dst_wallet, token_address, transfers)?;

		Ok(())
	}

	async fn erc1155_balance(&self, wallet_address: Address, token_address: Address, token_id: Uint) -> Uint {
		self.erc1155_wallet
			.read()
			.await
			.balance_of(wallet_address, token_address, token_id)
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
