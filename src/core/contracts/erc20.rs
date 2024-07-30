use crate::types::address::Address;
use ethabi::Uint;
use std::collections::HashMap;
use std::error::Error;

pub struct ERC20Wallet {
	balance: HashMap<(Address, Address), Uint>,
}

impl ERC20Wallet {
	pub fn new() -> Self {
		ERC20Wallet {
			balance: HashMap::new(),
		}
	}

	pub fn set_balance(&mut self, wallet_address: Address, token_address: Address, value: Uint) {
		if value.is_zero() {
			self.balance.remove(&(wallet_address, token_address));
		} else {
			self.balance.insert((wallet_address, token_address), value);
		}
	}

	pub fn balance_of(&self, wallet_address: Address, token_address: Address) -> Uint {
		self.balance
			.get(&(wallet_address, token_address))
			.cloned()
			.unwrap_or_else(Uint::zero)
	}

	pub fn transfer(
		&mut self,
		src_wallet: Address,
		dst_wallet: Address,
		token_address: Address,
		value: Uint,
	) -> Result<(), Box<dyn Error>> {
		if src_wallet == dst_wallet {
			return Err("can't transfer to self".into());
		}

		let new_src_balance = self
			.balance_of(src_wallet, token_address)
			.checked_sub(value)
			.ok_or("insufficient funds")?;
		let new_dst_balance = self
			.balance_of(dst_wallet, token_address)
			.checked_add(value)
			.ok_or("balance overflow")?;

		self.set_balance(src_wallet, token_address, new_src_balance);
		self.set_balance(dst_wallet, token_address, new_dst_balance);
		Ok(())
	}

	pub fn deposit(
		&mut self,
		wallet_address: Address,
		token_address: Address,
		value: Uint,
	) -> Result<(), Box<dyn Error>> {
		let new_balance = self.balance_of(wallet_address, token_address) + value;
		self.set_balance(wallet_address, token_address, new_balance);
		Ok(())
	}

	pub fn withdraw(
		&mut self,
		wallet_address: Address,
		token_address: Address,
		value: Uint,
	) -> Result<(), Box<dyn Error>> {
		let new_balance = self
			.balance_of(wallet_address, token_address)
			.checked_sub(value)
			.ok_or("insufficient funds")?;

		self.set_balance(wallet_address, token_address, new_balance);

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::address;

	#[test]
	fn test_erc20_wallet_initialization() {
		let wallet = ERC20Wallet::new();
		assert_eq!(wallet.balance, HashMap::new());
	}

	#[test]
	fn test_set_balance() {
		let mut wallet = ERC20Wallet::new();
		let wallet_address = address!("0x0000000000000000000000000000000000000001");
		let token_address = address!("0x0000000000000000000000000000000000000002");

		wallet.set_balance(wallet_address, token_address, Uint::from(100u64));
		assert_eq!(wallet.balance_of(wallet_address, token_address), Uint::from(100u64));

		wallet.set_balance(wallet_address, token_address, Uint::zero());
		assert_eq!(wallet.balance_of(wallet_address, token_address), Uint::zero());
	}

	#[test]
	fn test_transfer() {
		let mut wallet = ERC20Wallet::new();
		let src_wallet = address!("0x0000000000000000000000000000000000000001");
		let dst_wallet = address!("0x0000000000000000000000000000000000000002");
		let token_address = address!("0x0000000000000000000000000000000000000003");

		wallet.set_balance(src_wallet, token_address, Uint::from(100u64));
		wallet.set_balance(dst_wallet, token_address, Uint::from(50u64));

		let result = wallet.transfer(src_wallet, dst_wallet, token_address, Uint::from(30u64));
		assert!(result.is_ok());
		assert_eq!(wallet.balance_of(src_wallet, token_address), Uint::from(70u64));
		assert_eq!(wallet.balance_of(dst_wallet, token_address), Uint::from(80u64));
	}

	#[test]
	fn test_transfer_insufficient_funds() {
		let mut wallet = ERC20Wallet::new();
		let src_wallet = address!("0x0000000000000000000000000000000000000001");
		let dst_wallet = address!("0x0000000000000000000000000000000000000002");
		let token_address = address!("0x0000000000000000000000000000000000000003");

		wallet.set_balance(src_wallet, token_address, Uint::from(10u64));
		wallet.set_balance(dst_wallet, token_address, Uint::from(50u64));

		let result = wallet.transfer(src_wallet, dst_wallet, token_address, Uint::from(20u64));
		assert_eq!(result.unwrap_err().to_string(), "insufficient funds");
	}

	#[test]
	fn test_transfer_to_self() {
		let mut wallet = ERC20Wallet::new();
		let wallet_address = address!("0x0000000000000000000000000000000000000001");
		let token_address = address!("0x0000000000000000000000000000000000000002");

		wallet.set_balance(wallet_address, token_address, Uint::from(100u64));

		let result = wallet.transfer(wallet_address, wallet_address, token_address, Uint::from(10u64));
		assert_eq!(result.unwrap_err().to_string(), "can't transfer to self");
	}

	#[test]
	fn test_deposit() {
		let mut wallet = ERC20Wallet::new();
		let wallet_address = address!("0x0000000000000000000000000000000000000001");
		let token_address = address!("0x0000000000000000000000000000000000000002");
		let value = Uint::from(1_000_000_000_000_000_000u64);

		let result = wallet.deposit(wallet_address, token_address, value);

		assert!(result.is_ok());
		assert_eq!(wallet.balance_of(wallet_address, token_address), value);
	}

	#[test]
	fn test_withdraw() {
		let mut wallet = ERC20Wallet::new();
		let wallet_address = address!("0x0000000000000000000000000000000000000001");
		let token_address = address!("0x0000000000000000000000000000000000000002");

		wallet.set_balance(wallet_address, token_address, Uint::from(100u64));

		let result = wallet.withdraw(wallet_address, token_address, Uint::from(50u64));

		assert!(result.is_ok());
		assert_eq!(wallet.balance_of(wallet_address, token_address), Uint::from(50u64));
	}

	#[test]
	fn test_withdraw_insufficient_funds() {
		let mut wallet = ERC20Wallet::new();
		let wallet_address = address!("0x0000000000000000000000000000000000000001");
		let token_address = address!("0x0000000000000000000000000000000000000002");

		wallet.set_balance(wallet_address, token_address, Uint::from(10u64));

		let result = wallet.withdraw(wallet_address, token_address, Uint::from(50u64));
		assert_eq!(result.unwrap_err().to_string(), "insufficient funds");
	}
}
