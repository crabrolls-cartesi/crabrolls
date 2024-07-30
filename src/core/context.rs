use super::environment::Rollup;
use super::{application::Application, environment::RollupEnvironment};
use crate::{
	prelude::{Address, Deposit},
	types::machine::{FinishStatus, Input, PortalHandlerConfig},
	utils::address_book::AddressBook,
};
use std::error::Error;

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

pub async fn handle_portals<R: RollupEnvironment>(
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
pub struct Supervisor;

impl Supervisor {
	pub async fn run(app: impl Application, options: RunOptions) -> Result<(), Box<dyn Error>> {
		pretty_env_logger::init();

		let rollup = Rollup::new(options.rollup_url, options.address_book);

		let mut status = FinishStatus::Accept;

		println!(
			"Starting the application... Listening for inputs on {}",
			options.rollup_url
		);

		loop {
			let input = rollup.finish_and_get_next(status.clone()).await?;

			match input {
				Some(Input::Advance(advance_input)) => {
					debug!("New Advance input: {:?}", advance_input);

					if advance_input.metadata.sender == rollup.get_address_book().app_address_relay {
						debug!("Advance input from AppAddressRelay({})", advance_input.metadata.sender);
						let new_app_address: Address = advance_input.payload.clone().try_into()?;
						rollup.set_app_address(new_app_address).await;

						continue;
					}

					let deposit: Option<Deposit> =
						handle_portals(&rollup, advance_input.metadata.sender, advance_input.payload.clone()).await?;

					if deposit.is_some() {
						debug!("Deposited: {:?}", deposit);
					}

					match app
						.advance(&rollup, advance_input.metadata, &advance_input.payload, deposit)
						.await
					{
						Ok(result_status) => {
							debug!("Advance status: {:?}", result_status);
							status = result_status;
						}
						Err(e) => {
							error!("Error in advance: {}", e);
							status = FinishStatus::Reject;
						}
					}
				}
				Some(Input::Inspect(inspect_input)) => {
					debug!("Inspect input: {:?}", inspect_input);
					match app.inspect(&rollup, &inspect_input.payload).await {
						Ok(result_status) => {
							debug!("Inspect status: {:?}", result_status);
							status = result_status;
						}
						Err(e) => {
							error!("Error in inspect: {}", e);
							status = FinishStatus::Reject;
						}
					}
				}
				None => {
					debug!("Waiting for next input");
				}
			}
		}
	}
}
