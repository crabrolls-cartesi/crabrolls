use super::environment::Rollup;
use super::{application::Application, environment::RollupExtraEnvironment};
use crate::types::machine::{Advance, Inspect};
use crate::{
	prelude::{Address, Deposit},
	types::machine::{FinishStatus, Input, PortalHandlerConfig},
	utils::address_book::AddressBook,
};
use std::error::Error;

#[derive(Debug, Clone)]
pub struct RunOptions {
	pub rollup_url: &'static str,
	pub address_book: AddressBook,
	pub portal_config: PortalHandlerConfig,
}

impl Default for RunOptions {
	fn default() -> Self {
		Self {
			rollup_url: "http://127.0.0.1:5004",
			address_book: AddressBook::default(),
			portal_config: PortalHandlerConfig::default(),
		}
	}
}

impl RunOptions {
	pub fn builder() -> RunOptionsBuilder {
		RunOptionsBuilder::default()
	}
}

pub struct RunOptionsBuilder {
	rollup_url: &'static str,
	address_book: AddressBook,
	portal_config: PortalHandlerConfig,
}

impl Default for RunOptionsBuilder {
	fn default() -> Self {
		Self {
			rollup_url: "http://127.0.0.1:5004",
			address_book: AddressBook::default(),
			portal_config: PortalHandlerConfig::default(),
		}
	}
}

impl RunOptionsBuilder {
	pub fn rollup_url(mut self, rollup_url: &'static str) -> Self {
		self.rollup_url = rollup_url;
		self
	}

	pub fn address_book(mut self, address_book: AddressBook) -> Self {
		self.address_book = address_book;
		self
	}

	pub fn portal_config(mut self, portal_config: PortalHandlerConfig) -> Self {
		self.portal_config = portal_config;
		self
	}

	pub fn build(self) -> RunOptions {
		RunOptions {
			rollup_url: self.rollup_url,
			address_book: self.address_book,
			portal_config: self.portal_config,
		}
	}
}

pub async fn handle_portals<R: RollupExtraEnvironment>(
	rollup: &R,
	sender: Address,
	payload: Vec<u8>,
) -> Result<Option<Deposit>, Box<dyn Error>> {
	match sender {
		sender if sender == rollup.get_address_book().ether_portal => {
			debug!("Advance input from EtherPortal({})", sender);
			let (ether_deposit, _) = rollup.get_ether_wallet().write().await.deposit(payload.clone())?;
			Ok(Some(ether_deposit))
		}
		sender if sender == rollup.get_address_book().erc20_portal => {
			debug!("Advance input from ERC20Portal({})", sender);
			Ok(None)
		}
		sender if sender == rollup.get_address_book().erc721_portal => {
			debug!("Advance input from ERC721Portal({})", sender);
			Ok(None)
		}
		sender if sender == rollup.get_address_book().erc1155_single_portal => {
			debug!("Advance input from ERC1155SinglePortal({})", sender);
			Ok(None)
		}
		sender if sender == rollup.get_address_book().erc1155_batch_portal => {
			debug!("Advance input from ERC1155BatchPortal({})", sender);
			Ok(None)
		}
		_ => {
			debug!("Advance input from an unknown address");
			Ok(None)
		}
	}
}

pub fn is_portal<R: RollupExtraEnvironment>(rollup: &R, sender: Address) -> bool {
	sender == rollup.get_address_book().ether_portal
		|| sender == rollup.get_address_book().erc20_portal
		|| sender == rollup.get_address_book().erc721_portal
		|| sender == rollup.get_address_book().erc1155_single_portal
		|| sender == rollup.get_address_book().erc1155_batch_portal
}
pub struct Supervisor;

impl Supervisor {
	pub async fn run(app: impl Application, options: RunOptions) -> Result<(), Box<dyn Error>> {
		pretty_env_logger::init();
		let rollup = Rollup::new(options.rollup_url, options.address_book.clone());
		let mut status = FinishStatus::Accept;

		println!(
			"Starting the application... Listening for inputs on {}",
			options.rollup_url
		);

		loop {
			let input = rollup.finish_and_get_next(status.clone()).await?;

			match input {
				Some(Input::Advance(advance_input)) => {
					status = Self::handle_advance_input(&rollup, &options, &app, advance_input).await?;
				}
				Some(Input::Inspect(inspect_input)) => {
					status = Self::handle_inspect_input(&rollup, &app, inspect_input).await?;
				}
				None => {
					debug!("Waiting for next input");
				}
			}
		}
	}

	async fn handle_advance_input(
		rollup: &Rollup,
		options: &RunOptions,
		app: &impl Application,
		advance_input: Advance,
	) -> Result<FinishStatus, Box<dyn Error>> {
		debug!("New Advance input: {:?}", advance_input);

		if advance_input.metadata.sender == rollup.get_address_book().app_address_relay {
			debug!("Advance input from AppAddressRelay({})", advance_input.metadata.sender);
			let new_app_address: Address = advance_input.payload.clone().try_into()?;
			rollup.set_app_address(new_app_address).await;
			return Ok(FinishStatus::Accept);
		}

		let mut deposit: Option<Deposit> = None;

		if let PortalHandlerConfig::Handle { .. } = options.portal_config {
			deposit = handle_portals(rollup, advance_input.metadata.sender, advance_input.payload.clone()).await?;
		} else if is_portal(rollup, advance_input.metadata.sender)
			&& options.portal_config == PortalHandlerConfig::Dispense
		{
			debug!("Dispensing the deposit and discarding the advance input");
			return Ok(FinishStatus::Accept);
		}

		if deposit.is_some() {
			debug!("Deposited: {:?}", deposit);

			if options.portal_config == (PortalHandlerConfig::Handle { advance: false }) {
				return Ok(FinishStatus::Accept);
			}
		}

		match app
			.advance(rollup, advance_input.metadata, &advance_input.payload, deposit)
			.await
		{
			Ok(result_status) => {
				debug!("Advance status: {:?}", result_status);
				Ok(result_status)
			}
			Err(e) => {
				error!("Error in advance: {}", e);
				Ok(FinishStatus::Reject)
			}
		}
	}

	async fn handle_inspect_input(
		rollup: &Rollup,
		app: &impl Application,
		inspect_input: Inspect,
	) -> Result<FinishStatus, Box<dyn Error>> {
		debug!("Inspect input: {:?}", inspect_input);
		match app.inspect(rollup, &inspect_input.payload).await {
			Ok(result_status) => {
				debug!("Inspect status: {:?}", result_status);
				Ok(result_status)
			}
			Err(e) => {
				error!("Error in inspect: {}", e);
				Ok(FinishStatus::Reject)
			}
		}
	}
}
