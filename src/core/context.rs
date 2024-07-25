use super::application::Application;
use super::environment::Rollup;
use crate::types::machine::{FinishStatus, Input};
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

    let rollup = Rollup::new(options.rollup_url.clone());

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

                match advance_input.metadata.sender {
                    sender if sender == rollup.address_book.ether_portal => {
                        debug!("Advance input from EtherPortal({})", sender);
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
                    }
                    _ => {
                        debug!("Advance input from an unknown address");
                    }
                }

                match app
                    .advance(
                        &rollup,
                        advance_input.metadata,
                        advance_input.payload.as_slice(),
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
