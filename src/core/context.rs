use super::application::Application;
use super::environment::Rollup;
use crate::{
	prelude::{Address, Deposit},
	types::machine::{FinishStatus, Input},
};
use std::error::Error;

pub struct RunOptions {
	pub rollup_url: String,
}

impl Default for RunOptions {
	fn default() -> Self {
		Self {
			rollup_url: String::from("http://127.0.0.1:5004"),
		}
	}
}

impl RunOptions {
	pub fn new(rollup_url: String) -> Self {
		Self { rollup_url }
	}
}

pub async fn run(app: impl Application, options: RunOptions) -> Result<(), Box<dyn Error>> {
	pretty_env_logger::init();

	let mut rollup = Rollup::new(options.rollup_url.clone());

	let mut status = FinishStatus::Accept;

	println!(
		"Starting the application... Listening for inputs on {}",
		options.rollup_url
	);

	loop {
		let input = rollup.finish_and_get_next(status.clone()).await?;

		match input {
			Some(Input::Advance(advance_input)) => {
				debug!("NNew Advance input: {:?}", advance_input);

				let mut deposit: Option<Deposit> = None;

				match advance_input.metadata.sender {
					sender if sender == rollup.address_book.ether_portal => {
						debug!("Advance input from EtherPortal({})", sender);
						let (ether_deposit, _) = rollup
							.ether_wallet
							.write()
							.await
							.deposit(advance_input.payload.clone())?;
						deposit = Some(ether_deposit);
					}
					sender if sender == rollup.address_book.erc20_portal => {
						debug!("Advance input from ERC20Portal({})", sender);
					}
					sender if sender == rollup.address_book.erc721_portal => {
						debug!("Advance input from ERC721Portal({})", sender);
					}
					sender if sender == rollup.address_book.erc1155_single_portal => {
						debug!("Advance input from ERC1155SinglePortal({})", sender);
					}
					sender if sender == rollup.address_book.erc1155_batch_portal => {
						debug!("Advance input from ERC1155BatchPortal({})", sender);
					}
					sender if sender == rollup.address_book.app_address_relay => {
						debug!("Advance input from AppAddressRelay({})", sender);
						let new_app_address: Address = advance_input.payload.clone().try_into()?;
						rollup.set_app_address(new_app_address);
						continue;
					}
					_ => {
						debug!("Advance input from an unknown address");
					}
				}

				if deposit.is_some() {
					debug!("Deposited: {:?}", deposit);
				}

				match app
					.advance(
						&rollup,
						advance_input.metadata,
						advance_input.payload.as_slice(),
						deposit,
					)
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
