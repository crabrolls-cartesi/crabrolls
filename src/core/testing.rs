use async_std::sync::{Mutex, RwLock};
use ethabi::Uint;
use std::{error::Error, sync::Arc, time::UNIX_EPOCH};

use crate::{
	types::{
		address::Address,
		machine::{Deposit, FinishStatus, Output, PortalHandlerConfig},
		testing::{AdvanceResult, InspectResult},
	},
	utils::address_book::AddressBook,
	Application, Environment, Metadata,
};

use super::{
	context::handle_portals,
	contracts::ether::{EtherEnvironment, EtherWallet},
	environment::RollupEnvironment,
};

pub struct RollupMockup {
	outputs: RwLock<Vec<Output>>,
	input_index: Mutex<u64>,

	address_book: AddressBook,
	ether_wallet: Arc<RwLock<EtherWallet>>,
}

impl RollupMockup {
	pub fn new() -> Self {
		RollupMockup {
			outputs: RwLock::new(Vec::new()),
			input_index: Mutex::new(0),
			address_book: AddressBook::default(),

			ether_wallet: Arc::new(RwLock::new(EtherWallet::new())),
		}
	}

	pub async fn handle(&self, output: Output) -> Result<i32, Box<dyn Error>> {
		let mut outputs = self.outputs.write().await;
		outputs.push(output);
		Ok(outputs.len().try_into()?)
	}

	async fn reset(&self) {
		let mut outputs = self.outputs.write().await;
		outputs.clear();
	}

	pub async fn advance(&self, status: FinishStatus) -> Result<Option<Vec<Output>>, Box<dyn Error>> {
		let mut input_index = self.input_index.lock().await;
		*input_index += 1;

		let outputs = self.outputs.read().await.clone();
		self.reset().await;

		match status {
			FinishStatus::Accept => Ok(Some(outputs)),
			FinishStatus::Reject => Ok(None),
		}
	}

	pub async fn get_input_index(&self) -> u64 {
		*self.input_index.lock().await
	}
}

impl Environment for RollupMockup {
	async fn send_voucher(
		&self,
		destination: Address,
		payload: impl AsRef<[u8]> + Send,
	) -> Result<i32, Box<dyn Error>> {
		self.handle(Output::Voucher {
			destination,
			payload: payload.as_ref().to_vec(),
		})
		.await
	}

	async fn send_notice(&self, payload: impl AsRef<[u8]> + Send) -> Result<i32, Box<dyn Error>> {
		self.handle(Output::Notice {
			payload: payload.as_ref().to_vec(),
		})
		.await
	}

	async fn send_report(&self, payload: impl AsRef<[u8]> + Send) -> Result<(), Box<dyn Error>> {
		self.handle(Output::Report {
			payload: payload.as_ref().to_vec(),
		})
		.await?;
		Ok(())
	}
}

impl EtherEnvironment for RollupMockup {
	async fn ether_addresses(&self) -> Vec<Address> {
		self.ether_wallet.read().await.addresses()
	}

	async fn ether_withdraw(&self, address: Address, value: Uint) -> Result<(), Box<dyn Error>> {
		let mut ether_wallet = self.ether_wallet.write().await;
		let _ = ether_wallet.withdraw(address, value)?;

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
pub struct MockupOptions {
	pub portal_config: PortalHandlerConfig,
}

impl Default for MockupOptions {
	fn default() -> Self {
		Self {
			portal_config: PortalHandlerConfig::default(),
		}
	}
}

impl MockupOptions {
	pub fn builder() -> MockupOptionsBuilder {
		MockupOptionsBuilder::default()
	}
}

pub struct MockupOptionsBuilder {
	portal_config: PortalHandlerConfig,
}

impl Default for MockupOptionsBuilder {
	fn default() -> Self {
		Self {
			portal_config: PortalHandlerConfig::default(),
		}
	}
}

impl MockupOptionsBuilder {
	pub fn portal_config(mut self, portal_config: PortalHandlerConfig) -> Self {
		self.portal_config = portal_config;
		self
	}

	pub fn build(self) -> MockupOptions {
		MockupOptions {
			portal_config: self.portal_config,
		}
	}
}

impl RollupEnvironment for RollupMockup {
	fn get_address_book(&self) -> AddressBook {
		self.address_book.clone()
	}

	fn get_ether_wallet(&self) -> Arc<RwLock<EtherWallet>> {
		self.ether_wallet.clone()
	}
}

pub struct Tester<A> {
	app: A,
	env: RollupMockup,
	mockup_options: MockupOptions,
}

impl<A> Tester<A>
where
	A: Application,
{
	pub fn new(app: A, mockup_options: MockupOptions) -> Self {
		Self {
			app,
			env: RollupMockup::new(),
			mockup_options,
		}
	}

	pub async fn deposit(&self, deposit: Deposit) -> AdvanceResult {
		let sender = match deposit {
			Deposit::Ether { .. } => self.env.address_book.ether_portal,
			Deposit::ERC20 { .. } => self.env.address_book.erc20_portal,
			Deposit::ERC721 { .. } => self.env.address_book.erc721_portal,
			Deposit::ERC1155Single { .. } => self.env.address_book.erc1155_single_portal,
			Deposit::ERC1155Batch { .. } => self.env.address_book.erc1155_batch_portal,
		};

		let metadata = Metadata {
			input_index: self.env.get_input_index().await,
			sender,
			block_number: self.env.get_input_index().await,
			timestamp: UNIX_EPOCH.elapsed().unwrap().as_secs(),
		};

		let deposit = handle_portals(&self.env, sender, deposit.into())
			.await
			.expect("Failed to handle deposit payload")
			.expect("No deposit returned");

		let (status, error) = match self.app.advance(&self.env, metadata.clone(), &[], Some(deposit)).await {
			Ok(finish_status) => (finish_status, None),
			Err(e) => (FinishStatus::Reject, Some(e)),
		};

		AdvanceResult {
			status,
			outputs: match self.env.advance(status).await {
				Ok(Some(outputs)) => outputs,
				_ => Vec::new(),
			},
			metadata,
			error,
		}
	}

	pub async fn advance<T>(&self, sender: Address, payload: T) -> AdvanceResult
	where
		T: Into<Vec<u8>>,
	{
		let metadata = Metadata {
			input_index: self.env.get_input_index().await,
			sender,
			block_number: self.env.get_input_index().await,
			timestamp: UNIX_EPOCH.elapsed().unwrap().as_secs(),
		};

		let (status, error) = match self
			.app
			.advance(&self.env, metadata.clone(), payload.into().as_ref(), None)
			.await
		{
			Ok(finish_status) => (finish_status, None),
			Err(e) => (FinishStatus::Reject, Some(e)),
		};

		AdvanceResult {
			status,
			outputs: match self.env.advance(status).await {
				Ok(Some(outputs)) => outputs,
				_ => Vec::new(),
			},
			metadata,
			error,
		}
	}

	pub async fn inspect(&self, payload: &[u8]) -> InspectResult {
		let (status, error) = match self.app.inspect(&self.env, payload).await {
			Ok(finish_status) => (finish_status, None),
			Err(e) => (FinishStatus::Reject, Some(e)),
		};

		InspectResult {
			status,
			outputs: self.env.outputs.read().await.clone(),
			error,
		}
	}

	pub async fn ether_addresses(&self) -> Vec<Address> {
		self.env.ether_addresses().await
	}

	pub async fn ether_withdraw(&self, address: Address, value: Uint) -> Result<(), Box<dyn Error>> {
		self.env.ether_withdraw(address, value).await
	}

	pub async fn ether_transfer(
		&self,
		source: Address,
		destination: Address,
		value: Uint,
	) -> Result<(), Box<dyn Error>> {
		self.env.ether_transfer(source, destination, value).await
	}

	pub async fn ether_balance(&self, address: Address) -> Uint {
		self.env.ether_balance(address).await
	}
}
