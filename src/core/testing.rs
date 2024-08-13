use async_std::sync::{Mutex, RwLock};
use ethabi::{Address, Uint};
use std::{error::Error, sync::Arc, time::UNIX_EPOCH};

use crate::{
	address,
	types::{
		machine::{Deposit, FinishStatus, Output, PortalHandlerConfig},
		testing::{AdvanceResult, InspectResult},
	},
	utils::address_book::AddressBook,
	Application, Environment, Metadata,
};

use super::{
	context::handle_portals,
	contracts::{
		erc1155::{ERC1155Environment, ERC1155Wallet, IntoIdsAmountsIter},
		erc20::{ERC20Environment, ERC20Wallet},
		erc721::{ERC721Environment, ERC721Wallet},
		ether::{EtherEnvironment, EtherWallet},
	},
	environment::RollupInternalEnvironment,
};

pub struct RollupMockup {
	outputs: RwLock<Vec<Output>>,
	input_index: Mutex<u64>,
	app_address: Address,
	address_book: AddressBook,

	ether_wallet: Arc<RwLock<EtherWallet>>,
	erc20_wallet: Arc<RwLock<ERC20Wallet>>,
	erc721_wallet: Arc<RwLock<ERC721Wallet>>,
	erc1155_wallet: Arc<RwLock<ERC1155Wallet>>,
}

impl RollupMockup {
	pub fn new() -> Self {
		RollupMockup {
			outputs: RwLock::new(Vec::new()),
			input_index: Mutex::new(0),
			address_book: AddressBook::default(),
			app_address: address!("0xab7528bb862fb57e8a2bcd567a2e929a0be56a5e"),
			ether_wallet: Arc::new(RwLock::new(EtherWallet::new())),
			erc20_wallet: Arc::new(RwLock::new(ERC20Wallet::new())),
			erc721_wallet: Arc::new(RwLock::new(ERC721Wallet::new())),
			erc1155_wallet: Arc::new(RwLock::new(ERC1155Wallet::new())),
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
		let payload = ether_wallet.withdraw(address, value)?;

		self.send_voucher(self.app_address, payload).await?;

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

impl ERC20Environment for RollupMockup {
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

impl ERC721Environment for RollupMockup {
	async fn erc721_addresses(&self) -> Vec<Address> {
		self.erc721_wallet.read().await.addresses()
	}

	async fn erc721_withdraw(
		&self,
		wallet_address: Address,
		token_address: Address,
		token_id: Uint,
	) -> Result<(), Box<dyn Error>> {
		let mut erc721_wallet = self.erc721_wallet.write().await;
		let payload = erc721_wallet.withdraw(self.app_address, wallet_address, token_address, token_id)?;

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

impl ERC1155Environment for RollupMockup {
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
		let mut erc1155_wallet = self.erc1155_wallet.write().await;
		let payload = erc1155_wallet.withdraw(self.app_address, wallet_address, token_address, withdrawals, data)?;

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

impl RollupInternalEnvironment for RollupMockup {
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
		let sender = match deposit.clone() {
			Deposit::Ether { .. } => self.env.address_book.ether_portal,
			Deposit::ERC20 { .. } => self.env.address_book.erc20_portal,
			Deposit::ERC721 { .. } => self.env.address_book.erc721_portal,
			Deposit::ERC1155 {
				ids_amounts,
				sender: _,
				token: _,
			} => {
				if ids_amounts.len() == 1 {
					self.env.address_book.erc1155_single_portal
				} else {
					self.env.address_book.erc1155_batch_portal
				}
			}
		};

		let metadata = Metadata {
			input_index: self.env.get_input_index().await,
			sender,
			block_number: self.env.get_input_index().await,
			timestamp: UNIX_EPOCH.elapsed().unwrap().as_secs(),
		};

		let (status, error) = match self.mockup_options.portal_config {
			PortalHandlerConfig::Dispense => (FinishStatus::Accept, None),
			PortalHandlerConfig::Ignore => {
				let payload: Vec<u8> = deposit.try_into().expect("Failed to convert deposit to payload");
				match self
					.app
					.advance(&self.env, metadata.clone(), payload.as_slice(), None)
					.await
				{
					Ok(finish_status) => (finish_status, None),
					Err(e) => (FinishStatus::Reject, Some(e)),
				}
			}
			PortalHandlerConfig::Handle { advance } => {
				let deposit_payload = handle_portals(
					&self.env,
					sender,
					deposit.try_into().expect("Failed to convert deposit to payload"),
				)
				.await
				.expect("Failed to handle deposit payload")
				.expect("No deposit returned");

				if advance {
					match self
						.app
						.advance(&self.env, metadata.clone(), &[], Some(deposit_payload))
						.await
					{
						Ok(finish_status) => (finish_status, None),
						Err(e) => (FinishStatus::Reject, Some(e)),
					}
				} else {
					(FinishStatus::Accept, None)
				}
			}
		};

		let outputs = match self.env.advance(status).await {
			Ok(Some(outputs)) => outputs,
			_ => Vec::new(),
		};

		AdvanceResult {
			status,
			outputs,
			metadata,
			error,
		}
	}

	pub async fn advance(&self, sender: Address, payload: impl AsRef<[u8]> + Send) -> AdvanceResult {
		let metadata = Metadata {
			input_index: self.env.get_input_index().await,
			sender,
			block_number: self.env.get_input_index().await,
			timestamp: UNIX_EPOCH.elapsed().unwrap().as_secs(),
		};

		let (status, error) = match self
			.app
			.advance(&self.env, metadata.clone(), payload.as_ref(), None)
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

	pub async fn inspect(&self, payload: impl AsRef<[u8]> + Send) -> InspectResult {
		let (status, error) = match self.app.inspect(&self.env, payload.as_ref()).await {
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

	pub async fn erc20_addresses(&self) -> Vec<Address> {
		self.env.erc20_addresses().await
	}

	pub async fn erc20_withdraw(
		&self,
		wallet_address: Address,
		token_address: Address,
		value: Uint,
	) -> Result<(), Box<dyn Error>> {
		self.env.erc20_withdraw(wallet_address, token_address, value).await
	}

	pub async fn erc20_transfer(
		&self,
		src_wallet: Address,
		dst_wallet: Address,
		token_address: Address,
		value: Uint,
	) -> Result<(), Box<dyn Error>> {
		self.env
			.erc20_transfer(src_wallet, dst_wallet, token_address, value)
			.await
	}

	pub async fn erc20_balance(&self, wallet_address: Address, token_address: Address) -> Uint {
		self.env.erc20_balance(wallet_address, token_address).await
	}

	pub async fn erc721_addresses(&self) -> Vec<Address> {
		self.env.erc721_addresses().await
	}

	pub async fn erc721_withdraw(
		&self,
		wallet_address: Address,
		token_address: Address,
		token_id: Uint,
	) -> Result<(), Box<dyn Error>> {
		self.env.erc721_withdraw(wallet_address, token_address, token_id).await
	}

	pub async fn erc721_transfer(
		&self,
		src_wallet: Address,
		dst_wallet: Address,
		token_address: Address,
		token_id: Uint,
	) -> Result<(), Box<dyn Error>> {
		self.env
			.erc721_transfer(src_wallet, dst_wallet, token_address, token_id)
			.await
	}

	pub async fn erc721_owner_of(&self, token_address: Address, token_id: Uint) -> Option<Address> {
		self.env.erc721_owner_of(token_address, token_id).await
	}

	pub async fn erc1155_addresses(&self) -> Vec<Address> {
		self.env.erc1155_addresses().await
	}

	pub async fn erc1155_withdraw<I>(
		&self,
		wallet_address: Address,
		token_address: Address,
		withdrawals: I,
		data: Option<Vec<u8>>,
	) -> Result<(), Box<dyn Error>>
	where
		I: IntoIdsAmountsIter,
	{
		self.env
			.erc1155_withdraw(wallet_address, token_address, withdrawals, data)
			.await
	}

	pub async fn erc1155_transfer<I>(
		&self,
		src_wallet: Address,
		dst_wallet: Address,
		token_address: Address,
		transfers: I,
	) -> Result<(), Box<dyn Error>>
	where
		I: IntoIdsAmountsIter,
	{
		self.env
			.erc1155_transfer(src_wallet, dst_wallet, token_address, transfers)
			.await
	}

	pub async fn erc1155_balance(&self, wallet_address: Address, token_address: Address, token_id: Uint) -> Uint {
		self.env.erc1155_balance(wallet_address, token_address, token_id).await
	}
}
