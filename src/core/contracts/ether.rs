use crate::types::address::Address;
use crate::utils::abi::encode;
use ethabi::Uint;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone)]
pub struct EtherDeposit {
	sender: Address,
	value: Uint,
}

impl EtherDeposit {
	pub fn new(sender: Address, value: Uint) -> Self {
		EtherDeposit { sender, value }
	}

	pub fn value_in_ether(&self) -> String {
		let ether_value = &self.value / &Uint::from(1_000_000_000_000_000_000u64);
		format!("{}", ether_value)
	}
}

impl fmt::Display for EtherDeposit {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"{} deposited {} Ether",
			self.sender.to_string(),
			self.value_in_ether()
		)
	}
}

pub struct EtherWallet {
	balance: HashMap<Address, Uint>,
}

impl EtherWallet {
	pub fn new() -> Self {
		EtherWallet {
			balance: HashMap::new(),
		}
	}

	pub fn addresses(&self) -> Vec<Address> {
		let mut addresses: Vec<Address> = self.balance.keys().cloned().collect();
		addresses.sort_by(|a, b| a.cmp(b));
		addresses
	}

	pub fn set_balance(&mut self, address: Address, value: Uint) {
		if value.is_zero() {
			self.balance.remove(&address);
		} else {
			self.balance.insert(address, value);
		}
	}

	pub fn balance_of(&self, address: Address) -> Uint {
		self.balance.get(&address).cloned().unwrap_or_else(|| Uint::zero())
	}

	pub fn deposit(&mut self, payload: &[u8]) -> Result<(EtherDeposit, Vec<u8>), Box<dyn Error>> {
		if payload.len() < 52 {
			return Err("invalid eth deposit size".into());
		}

		let sender = payload[0..20].into();
		let value = Uint::from_little_endian(&payload[20..52]);

		let new_balance = self.balance_of(sender) + value.clone();
		self.set_balance(sender, new_balance);

		let deposit = EtherDeposit::new(sender, value);
		Ok((deposit, payload[52..].to_vec()))
	}

	pub fn transfer(&mut self, src: Address, dst: Address, value: Uint) -> Result<(), Box<dyn Error>> {
		if src == dst {
			return Err("can't transfer to self".into());
		}

		let new_src_balance = self.balance_of(src).checked_sub(value).ok_or("insufficient funds")?;
		let new_dst_balance = self.balance_of(dst).checked_add(value).ok_or("balance overflow")?;

		self.set_balance(src, new_src_balance);
		self.set_balance(dst, new_dst_balance);
		Ok(())
	}

	pub fn withdraw(&mut self, address: Address, value: Uint) -> Result<Vec<u8>, Box<dyn Error>> {
		let new_balance = self
			.balance_of(address)
			.checked_sub(value)
			.ok_or("insufficient funds")?;

		if new_balance < Uint::zero() {
			return Err("insufficient funds".into());
		}

		self.set_balance(address, new_balance);

		Ok(encode::ether::withdraw(address, value)?)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::address;
	use crate::types::address::Address;

	#[test]
	fn test_ether_deposit_creation() {
		let address = address!("0x0000000000000000000000000000000000000001");
		let value = Uint::from(1_000_000_000_000_000_000u64);
		let deposit = EtherDeposit::new(address.clone(), value.clone());

		assert_eq!(deposit.sender, address);
		assert_eq!(deposit.value, value);
	}

	#[test]
	fn test_value_in_ether() {
		let address = address!("0x0000000000000000000000000000000000000001");
		let value = Uint::from(1_000_000_000_000_000_000u64);
		let deposit = EtherDeposit::new(address.clone(), value.clone());

		assert_eq!(deposit.value_in_ether(), "1");
	}

	#[test]
	fn test_ether_wallet_initialization() {
		let wallet = EtherWallet::new();
		assert_eq!(wallet.balance, HashMap::new());
	}

	#[test]
	fn test_addresses() {
		let mut wallet = EtherWallet::new();
		let addr1 = address!("0x0000000000000000000000000000000000000001");
		let addr2 = address!("0x0000000000000000000000000000000000000002");

		wallet.set_balance(addr2.clone(), Uint::from(10u64));
		wallet.set_balance(addr1.clone(), Uint::from(5u64));

		let addresses = wallet.addresses();
		assert_eq!(addresses, vec![addr1, addr2]);
	}

	#[test]
	fn test_set_balance() {
		let mut wallet = EtherWallet::new();
		let address = address!("0x0000000000000000000000000000000000000001");

		wallet.set_balance(address.clone(), Uint::from(100u64));
		assert_eq!(wallet.balance_of(address.clone()), Uint::from(100u64));

		wallet.set_balance(address.clone(), Uint::zero());
		assert_eq!(wallet.balance_of(address.clone()), Uint::zero());
	}

	#[test]
	fn test_transfer() {
		let mut wallet = EtherWallet::new();
		let src = address!("0x0000000000000000000000000000000000000001");
		let dst = address!("0x0000000000000000000000000000000000000002");

		wallet.set_balance(src.clone(), Uint::from(100u64));
		wallet.set_balance(dst.clone(), Uint::from(50u64));

		let result = wallet.transfer(src.clone(), dst.clone(), Uint::from(30u64));
		assert!(result.is_ok());
		assert_eq!(wallet.balance_of(src.clone()), Uint::from(70u64));
		assert_eq!(wallet.balance_of(dst.clone()), Uint::from(80u64));
	}

	#[test]
	fn test_transfer_insufficient_funds() {
		let mut wallet = EtherWallet::new();
		let src = address!("0x0000000000000000000000000000000000000001");
		let dst = address!("0x0000000000000000000000000000000000000002");

		wallet.set_balance(src.clone(), Uint::from(10u64));
		wallet.set_balance(dst.clone(), Uint::from(50u64));

		let result = wallet.transfer(src.clone(), dst.clone(), Uint::from(20u64));
		assert_eq!(result.unwrap_err().to_string(), "insufficient funds");
	}

	#[test]
	fn test_transfer_to_self() {
		let mut wallet = EtherWallet::new();
		let address = address!("0x0000000000000000000000000000000000000001");

		wallet.set_balance(address.clone(), Uint::from(100u64));

		let result = wallet.transfer(address.clone(), address.clone(), Uint::from(10u64));
		assert_eq!(result.unwrap_err().to_string(), "can't transfer to self");
	}

	#[test]
	fn test_withdraw() {
		let mut wallet = EtherWallet::new();
		let address = address!("0x0000000000000000000000000000000000000001");

		wallet.set_balance(address.clone(), Uint::from(100u64));

		let encoded_withdraw = wallet.withdraw(address.clone(), Uint::from(50u64)).unwrap();

		assert_eq!(wallet.balance_of(address.clone()), Uint::from(50u64));
	}

	#[test]
	fn test_withdraw_insufficient_funds() {
		let mut wallet = EtherWallet::new();
		let address = address!("0x0000000000000000000000000000000000000001");

		wallet.set_balance(address.clone(), Uint::from(10u64));

		let result = wallet.withdraw(address.clone(), Uint::from(50u64));
		assert_eq!(result.unwrap_err().to_string(), "insufficient funds");
	}

	#[test]
	fn test_deposit() {
		let mut wallet = EtherWallet::new();
		let address = address!("0x0000000000000000000000000000000000000001");
		let value = Uint::from(1_000_000_000_000_000_000u64);

		let mut value_bytes = vec![0u8; 32];
		value.to_little_endian(&mut value_bytes);

		let mut payload = vec![0u8; 52];
		payload[0..20].copy_from_slice(&address.0);
		payload[20..52].copy_from_slice(&value_bytes);

		let result = wallet.deposit(&payload);

		assert!(result.is_ok());

		let (deposit, remaining_payload) = result.unwrap();

		assert_eq!(deposit.sender, address);
		assert_eq!(deposit.value, value);

		assert_eq!(wallet.balance_of(address), value);

		assert!(remaining_payload.is_empty());
	}
}
