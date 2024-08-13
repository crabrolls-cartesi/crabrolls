use crabrolls::prelude::*;
use ethabi::{Address, Uint};
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind", content = "metadata")]
enum InspectBalance {
	#[serde(rename = "ether")]
	Ether { address: Address },
	#[serde(rename = "erc20")]
	ERC20 { address: Address, token: Address },
	#[serde(rename = "erc721")]
	ERC721 { token: Address, id: Uint },
	#[serde(rename = "erc1155")]
	ERC1155 { address: Address, token: Address, id: Uint },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind", content = "metadata")]
enum AdvanceWithdraw {
	#[serde(rename = "ether")]
	Ether {},
	#[serde(rename = "erc20")]
	ERC20 { token: Address },
	#[serde(rename = "erc721")]
	ERC721 { token: Address, id: Uint },
	#[serde(rename = "erc1155")]
	ERC1155 {
		token: Address,
		ids: Vec<Uint>,
		data: Option<Vec<u8>>,
	},
}

pub struct SuperHoneypotApp;

impl SuperHoneypotApp {
	pub fn new() -> Self {
		Self
	}
}

impl Application for SuperHoneypotApp {
	async fn advance(
		&self,
		env: &impl Environment,
		metadata: Metadata,
		payload: &[u8],
		deposit: Option<Deposit>,
	) -> Result<FinishStatus, Box<dyn Error>> {
		match deposit {
			Some(Deposit::Ether { sender, amount }) => {
				println!(
					"Received deposit of {} ether from {}",
					units::wei::to_ether(amount),
					sender
				);

				let balance = env.ether_balance(sender).await;
				println!("Current balance of sender: {} ether", units::wei::to_ether(balance));
			}
			Some(Deposit::ERC20 { sender, token, amount }) => {
				println!("Received deposit of {} ERC20 tokens from {}", amount, sender);

				let balance = env.erc20_balance(sender, token).await;
				println!("Current balance of sender's ERC20 tokens: {}", balance);
			}
			Some(Deposit::ERC721 { sender, token, id }) => {
				println!(
					"Received ERC721 token with ID {} from {}, with token address {}",
					id, sender, token
				);
			}
			Some(Deposit::ERC1155 {
				sender,
				token,
				ids_amounts,
			}) => {
				println!("Received ERC1155 deposit from {}", sender);

				for (id, _amount) in ids_amounts {
					let balance = env.erc1155_balance(sender, token, id).await;
					println!("Current balance of ERC1155 token ID {}: {}", id, balance);
				}
			}
			None => {
				println!("Received no deposit, triggering withdrawal request");

				let withdraw = match serde_json::from_slice::<AdvanceWithdraw>(payload) {
					Ok(withdraw) => withdraw,
					Err(e) => {
						println!("Error deserializing withdrawal request: {}", e);
						return Ok(FinishStatus::Reject);
					}
				};

				println!("Withdrawal request: {:?}", withdraw);

				match withdraw {
					AdvanceWithdraw::Ether {} => {
						let balance = env.ether_balance(metadata.sender).await;
						println!("Current balance of sender: {} ether", units::wei::to_ether(balance));

						if balance != Uint::zero() {
							println!("Withdrawing all ether from account");
							env.ether_withdraw(metadata.sender, balance).await?;
						} else {
							println!("Sender has no ether to withdraw.");
						}
					}
					AdvanceWithdraw::ERC20 { token } => {
						let balance = env.erc20_balance(metadata.sender, token).await;
						println!("Current balance of sender's ERC20 tokens: {}", balance);

						if balance != Uint::zero() {
							println!("Withdrawing all ERC20 tokens from account");
							env.erc20_withdraw(metadata.sender, token, balance).await?;
						} else {
							println!("Sender has no ERC20 tokens to withdraw.");
						}
					}
					AdvanceWithdraw::ERC721 { token, id } => {
						let is_owner = env.erc721_owner_of(token, id).await == Some(metadata.sender);

						if is_owner {
							println!("Withdrawing ERC721 token with ID {} from sender", id);
							env.erc721_withdraw(metadata.sender, token, id).await?;
						} else {
							println!("Sender is not the owner of the ERC721 token with ID {}", id);
						}
					}
					AdvanceWithdraw::ERC1155 { token, ids, data } => {
						let mut ids_balance = Vec::new();

						for id in &ids {
							let balance = env.erc1155_balance(metadata.sender, token, *id).await;
							println!("Current balance of ERC1155 token ID {}: {}", id, balance);

							if balance == Uint::zero() {
								println!(
									"Sender has no ERC1155 tokens of ID {} to withdraw. Continuing to next ID.",
									id
								);
								break;
							}

							ids_balance.push((*id, balance));
						}

						if ids_balance.is_empty() {
							println!("Sender has no ERC1155 tokens to withdraw.");
						} else {
							println!(
								"Withdrawing all balance of ERC1155 tokens {:?} from account",
								ids_balance.iter().map(|(id, _)| id).collect::<Vec<_>>()
							);
							env.erc1155_withdraw(metadata.sender, token, ids_balance, data).await?;
						}
					}
				}
			}
		}

		Ok(FinishStatus::Accept)
	}

	async fn inspect(&self, env: &impl Environment, payload: &[u8]) -> Result<FinishStatus, Box<dyn Error>> {
		let inspect = match serde_json::from_slice::<InspectBalance>(payload) {
			Ok(inspect) => inspect,
			Err(e) => {
				println!("Error deserializing inspection request: {}", e);
				return Ok(FinishStatus::Reject);
			}
		};

		println!("Inspection request: {:?}", inspect);

		match inspect {
			InspectBalance::Ether { address } => {
				let balance = env.ether_balance(address).await;
				println!(
					"Balance of ether for address {}: {}",
					address,
					units::wei::to_ether(balance)
				);

				env.send_report(balance.to_string()).await?;
			}
			InspectBalance::ERC20 { address, token } => {
				let balance = env.erc20_balance(address, token).await;
				println!("Balance of ERC20 token {} for address {}: {}", token, address, balance);

				env.send_report(balance.to_string()).await?;
			}
			InspectBalance::ERC721 { token, id } => {
				let owner = env.erc721_owner_of(token, id).await;

				if owner.is_none() {
					println!("ERC721 token ID {} does not exist", id);
					env.send_report("0").await?;
				} else {
					println!("Owner of ERC721 token ID {}: {}", id, owner.unwrap());
				}

				env.send_report(owner.to_owned().unwrap_or(Address::zero())).await?;
			}
			InspectBalance::ERC1155 { address, token, id } => {
				let balance = env.erc1155_balance(address, token, id).await;
				println!(
					"Balance of ERC1155 token ID {} for address {}: {}",
					id, address, balance
				);

				env.send_report(balance.to_string()).await?;
			}
		}
		Ok(FinishStatus::Accept)
	}
}

#[async_std::main]
async fn main() {
	let app = SuperHoneypotApp::new();
	let options = RunOptions::default();
	if let Err(e) = Supervisor::run(app, options).await {
		eprintln!("Error: {}", e);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[async_std::test]
	async fn test_ether_deposit_and_withdrawal() {
		let app = SuperHoneypotApp::new();
		let tester = Tester::new(app, MockupOptions::default());

		let address = Address::default();
		let amount = units::wei::from_ether(6.0);

		let deposit_result = tester
			.deposit(Deposit::Ether {
				sender: address,
				amount,
			})
			.await;

		assert!(deposit_result.is_accepted(), "Expected Accept status");
		assert_eq!(tester.ether_balance(address).await, amount);

		let withdraw_payload = json!({
			"kind": "ether",
			"metadata": {}
		})
		.to_string();

		let advance_result = tester.advance(address, withdraw_payload).await;

		assert!(advance_result.is_accepted(), "Expected Accept status");
		assert_eq!(tester.ether_balance(address).await, Uint::zero());
		assert_eq!(advance_result.get_outputs().len(), 1);
	}

	#[async_std::test]
	async fn test_erc20_deposit_and_withdrawal() {
		let app = SuperHoneypotApp::new();
		let tester = Tester::new(app, MockupOptions::default());

		let address = Address::default();
		let token_address = Address::from_low_u64_be(1);
		let amount = Uint::from(1000u64);

		let deposit_result = tester
			.deposit(Deposit::ERC20 {
				sender: address,
				token: token_address,
				amount,
			})
			.await;

		assert!(deposit_result.is_accepted(), "Expected Accept status");
		assert_eq!(tester.erc20_balance(address, token_address).await, amount);

		let withdraw_payload = json!({
			"kind": "erc20",
			"metadata": {
				"token": token_address
			}
		})
		.to_string();

		let advance_result = tester.advance(address, withdraw_payload).await;

		assert!(advance_result.is_accepted(), "Expected Accept status");
		assert_eq!(tester.erc20_balance(address, token_address).await, Uint::zero());
		assert_eq!(advance_result.get_outputs().len(), 1);
	}

	#[async_std::test]
	async fn test_erc721_deposit_and_withdrawal() {
		let app = SuperHoneypotApp::new();
		let tester = Tester::new(app, MockupOptions::default());

		let address = Address::default();
		let token_address = Address::from_low_u64_be(1);
		let token_id = Uint::from(1u64);

		let deposit_result = tester
			.deposit(Deposit::ERC721 {
				sender: address,
				token: token_address,
				id: token_id,
			})
			.await;

		assert!(deposit_result.is_accepted(), "Expected Accept status");

		let withdraw_payload = json!({
			"kind": "erc721",
			"metadata": {
				"token": token_address,
				"id": token_id
			}
		})
		.to_string();

		let advance_result = tester.advance(address, withdraw_payload).await;

		assert!(advance_result.is_accepted(), "Expected Accept status");
		assert_eq!(tester.erc721_owner_of(token_address, token_id).await, None);
		assert_eq!(advance_result.get_outputs().len(), 1);
	}

	#[async_std::test]
	async fn test_erc1155_deposit_and_withdrawal() {
		let app = SuperHoneypotApp::new();
		let tester = Tester::new(app, MockupOptions::default());

		let address = Address::default();
		let token_address = Address::from_low_u64_be(1);
		let token_id = Uint::from(1u64);
		let amount = Uint::from(10u64);

		let deposit_result = tester
			.deposit(Deposit::ERC1155 {
				sender: address,
				token: token_address,
				ids_amounts: vec![(token_id, amount)],
			})
			.await;

		assert!(deposit_result.is_accepted(), "Expected Accept status");
		assert_eq!(tester.erc1155_balance(address, token_address, token_id).await, amount);

		let withdraw_payload = json!({
			"kind": "erc1155",
			"metadata": {
				"token": token_address,
				"ids": [token_id],
				"data": null
			}
		})
		.to_string();

		let advance_result = tester.advance(address, withdraw_payload).await;

		assert!(advance_result.is_accepted(), "Expected Accept status");
		assert_eq!(
			tester.erc1155_balance(address, token_address, token_id).await,
			Uint::zero()
		);
		assert_eq!(advance_result.get_outputs().len(), 1);
	}

	#[async_std::test]
	async fn test_inspect_ether_balance() {
		let app = SuperHoneypotApp::new();
		let tester = Tester::new(app, MockupOptions::default());

		let address = Address::default();
		let amount = units::wei::from_ether(10.0);

		tester
			.deposit(Deposit::Ether {
				sender: address,
				amount,
			})
			.await;

		let inspect_payload = json!({
			"kind": "ether",
			"metadata": {
				"address": address
			}
		})
		.to_string();

		let inspect_result = tester.inspect(inspect_payload).await;

		assert!(inspect_result.is_accepted(), "Expected Accept status");
		assert_eq!(
			inspect_result.get_outputs(),
			vec![Output::Report {
				payload: amount.to_string().to_vec()
			}]
		);
	}

	#[async_std::test]
	async fn test_inspect_erc20_balance() {
		let app = SuperHoneypotApp::new();
		let tester = Tester::new(app, MockupOptions::default());

		let address = Address::default();
		let token_address = Address::from_low_u64_be(1);
		let amount = Uint::from(1000u64);

		tester
			.deposit(Deposit::ERC20 {
				sender: address,
				token: token_address,
				amount,
			})
			.await;

		let inspect_payload = json!({
			"kind": "erc20",
			"metadata": {
				"address": address,
				"token": token_address
			}
		})
		.to_string();

		let inspect_result = tester.inspect(inspect_payload).await;

		assert!(inspect_result.is_accepted(), "Expected Accept status");
		assert_eq!(
			inspect_result.get_outputs(),
			vec![Output::Report {
				payload: amount.to_string().to_vec()
			}]
		);
	}

	#[async_std::test]
	async fn test_inspect_erc721_owner() {
		let app = SuperHoneypotApp::new();
		let tester = Tester::new(app, MockupOptions::default());

		let address = Address::default();
		let token_address = Address::from_low_u64_be(1);
		let token_id = Uint::from(1u64);

		tester
			.deposit(Deposit::ERC721 {
				sender: address,
				token: token_address,
				id: token_id,
			})
			.await;

		let inspect_payload = json!({
			"kind": "erc721",
			"metadata": {
				"token": token_address,
				"id": token_id
			}
		})
		.to_string();

		let inspect_result = tester.inspect(inspect_payload).await;

		assert!(inspect_result.is_accepted(), "Expected Accept status");
		assert_eq!(
			inspect_result.get_outputs(),
			vec![Output::Report {
				payload: address.to_vec()
			}]
		);
	}

	#[async_std::test]
	async fn test_inspect_erc1155_balance() {
		let app = SuperHoneypotApp::new();
		let tester = Tester::new(app, MockupOptions::default());

		let address = Address::default();
		let token_address = Address::from_low_u64_be(1);
		let token_id = Uint::from(1u64);
		let amount = Uint::from(10u64);

		tester
			.deposit(Deposit::ERC1155 {
				sender: address,
				token: token_address,
				ids_amounts: vec![(token_id, amount)],
			})
			.await;

		let inspect_payload = json!({
			"kind": "erc1155",
			"metadata": {
				"address": address,
				"token": token_address,
				"id": token_id
			}
		})
		.to_string();

		let inspect_result = tester.inspect(inspect_payload).await;

		assert!(inspect_result.is_accepted(), "Expected Accept status");
		assert_eq!(
			inspect_result.get_outputs(),
			vec![Output::Report {
				payload: amount.to_string().to_vec()
			}]
		);
	}
}
