use crabrolls::prelude::*;
use std::error::Error;

struct DepositApp;

impl DepositApp {
	fn new() -> Self {
		Self
	}
}

impl Application for DepositApp {
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

				println!(
					"Balance of sender: {}",
					units::wei::to_ether(env.ether_balance(sender).await)
				);
				if units::wei::to_ether(env.ether_balance(sender).await) > 10.0 {
					env.ether_withdraw(sender, env.ether_balance(sender).await).await?;
				}
			}
			None => {}
			_ => todo!(),
		}

		Ok(FinishStatus::Accept)
	}

	async fn inspect(&self, env: &impl Environment, payload: &[u8]) -> Result<FinishStatus, Box<dyn Error>> {
		println!(
			"Inspect method called with payload: {:?}",
			String::from_utf8_lossy(&payload)
		);
		Ok(FinishStatus::Accept)
	}
}

#[async_std::main]
async fn main() {
	let app = DepositApp::new();
	let options = RunOptions::default();
	if let Err(e) = Supervisor::run(app, options).await {
		eprintln!("Error: {}", e);
	}
}

#[cfg(test)]
mod tests {
	use super::DepositApp;
	use crabrolls::prelude::*;

	#[async_std::test]
	async fn test_echo() {
		let app = DepositApp::new();
		let tester = Tester::new(app, MockupOptions::default());

		let address = Address::default();

		let result = tester
			.deposit(Deposit::Ether {
				sender: address,
				amount: units::wei::from_ether(6.0),
			})
			.await;

		assert_eq!(result.status, FinishStatus::Accept, "Expected Accept status");
		assert_eq!(tester.ether_balance(address).await, units::wei::from_ether(6.0));
		assert_eq!(tester.ether_addresses().await, vec![address]);

		let result = tester
			.deposit(Deposit::Ether {
				sender: address,
				amount: units::wei::from_ether(6.0),
			})
			.await;

		assert_eq!(result.status, FinishStatus::Accept, "Expected Accept status");
		assert_eq!(tester.ether_balance(address).await, units::wei::from_ether(0.0));
		assert_eq!(tester.ether_addresses().await, vec![]);
		assert_eq!(result.outputs.len(), 1);
	}
}
