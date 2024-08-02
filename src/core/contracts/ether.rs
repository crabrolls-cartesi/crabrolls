use crate::types::address::Address;
use crate::types::machine::Deposit;
use crate::utils::abi::abi;
use ethabi::Uint;
use std::collections::HashMap;
use std::error::Error;
use std::future::Future;

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

	pub fn deposit(&mut self, payload: Vec<u8>) -> Result<(Deposit, Vec<u8>), Box<dyn Error>> {
		let args = abi::ether::deposit(payload.clone())?;

		let sender: Address = abi::extract::address(&args[0])?;
		let value: Uint = abi::extract::uint(&args[1])?;

		debug!("new ether deposit from {:?} with value {:?}", sender, value);

		let new_balance = self.balance_of(sender) + value;
		self.set_balance(sender, new_balance);

		let deposit = Deposit::Ether { sender, amount: value };
		Ok((deposit, payload[abi::utils::size_of_packed_tokens(&args)..].to_vec()))
	}

	pub fn deposit_payload(sender: Address, value: Uint) -> Vec<u8> {
		let mut value_bytes = vec![0u8; 32];
		value.to_big_endian(&mut value_bytes);

		let mut payload = vec![0u8; 52];
		payload[0..20].copy_from_slice(&sender.0);
		payload[20..52].copy_from_slice(&value_bytes);

		payload
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

		Ok(abi::ether::withdraw(address, value)?)
	}
}

pub trait EtherEnvironment {
	fn ether_addresses(&self) -> impl Future<Output = Vec<Address>>;
	fn ether_withdraw(&self, address: Address, value: Uint) -> impl Future<Output = Result<(), Box<dyn Error>>>;
	fn ether_transfer(
		&self,
		source: Address,
		destination: Address,
		value: Uint,
	) -> impl Future<Output = Result<(), Box<dyn Error>>>;
	fn ether_balance(&self, address: Address) -> impl Future<Output = Uint>;
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
		let deposit = Deposit::Ether {
			sender: address,
			amount: value,
		};

		if let Deposit::Ether { sender, amount } = deposit {
			assert_eq!(sender, address);
			assert_eq!(amount, value);
		} else {
			panic!("invalid deposit type");
		}
	}

	#[test]
	fn test_value_in_ether() {
		let address = address!("0x0000000000000000000000000000000000000001");
		let value = Uint::from(1_000_000_000_000_000_000u64);
		let deposit = Deposit::Ether {
			sender: address,
			amount: value,
		};

		if let Deposit::Ether { sender, amount } = deposit {
			assert_eq!(sender, address);
			assert_eq!(amount, value);
		} else {
			panic!("invalid deposit type");
		}
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

		wallet.set_balance(addr2, Uint::from(10u64));
		wallet.set_balance(addr1, Uint::from(5u64));

		let addresses = wallet.addresses();
		assert_eq!(addresses, vec![addr1, addr2]);
	}

	#[test]
	fn test_set_balance() {
		let mut wallet = EtherWallet::new();
		let address = address!("0x0000000000000000000000000000000000000001");

		wallet.set_balance(address, Uint::from(100u64));
		assert_eq!(wallet.balance_of(address), Uint::from(100u64));

		wallet.set_balance(address, Uint::zero());
		assert_eq!(wallet.balance_of(address), Uint::zero());
	}

	#[test]
	fn test_transfer() {
		let mut wallet = EtherWallet::new();
		let src = address!("0x0000000000000000000000000000000000000001");
		let dst = address!("0x0000000000000000000000000000000000000002");

		wallet.set_balance(src, Uint::from(100u64));
		wallet.set_balance(dst, Uint::from(50u64));

		let result = wallet.transfer(src, dst, Uint::from(30u64));
		assert!(result.is_ok());
		assert_eq!(wallet.balance_of(src), Uint::from(70u64));
		assert_eq!(wallet.balance_of(dst), Uint::from(80u64));
	}

	#[test]
	fn test_transfer_insufficient_funds() {
		let mut wallet = EtherWallet::new();
		let src = address!("0x0000000000000000000000000000000000000001");
		let dst = address!("0x0000000000000000000000000000000000000002");

		wallet.set_balance(src, Uint::from(10u64));
		wallet.set_balance(dst, Uint::from(50u64));

		let result = wallet.transfer(src, dst, Uint::from(20u64));
		assert_eq!(result.unwrap_err().to_string(), "insufficient funds");
	}

	#[test]
	fn test_transfer_to_self() {
		let mut wallet = EtherWallet::new();
		let address = address!("0x0000000000000000000000000000000000000001");

		wallet.set_balance(address, Uint::from(100u64));

		let result = wallet.transfer(address, address, Uint::from(10u64));
		assert_eq!(result.unwrap_err().to_string(), "can't transfer to self");
	}

	#[test]
	fn test_withdraw() {
		let mut wallet = EtherWallet::new();
		let address = address!("0x0000000000000000000000000000000000000001");

		wallet.set_balance(address, Uint::from(100u64));

		let encoded_withdraw = wallet.withdraw(address, Uint::from(50u64)).unwrap();

		assert_eq!(wallet.balance_of(address), Uint::from(50u64));
		assert_eq!(encoded_withdraw.len(), 68);
	}

	#[test]
	fn test_withdraw_insufficient_funds() {
		let mut wallet = EtherWallet::new();
		let address = address!("0x0000000000000000000000000000000000000001");

		wallet.set_balance(address, Uint::from(10u64));

		let result = wallet.withdraw(address, Uint::from(50u64));
		assert_eq!(result.unwrap_err().to_string(), "insufficient funds");
	}

	#[test]
	fn test_deposit() {
		let mut wallet = EtherWallet::new();
		let address = address!("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
		let value = Uint::from(100);

		let mut value_bytes = vec![0u8; 32];
		value.to_big_endian(&mut value_bytes);

		let mut payload = vec![0u8; 52];
		payload[0..20].copy_from_slice(address.as_ref());
		payload[20..52].copy_from_slice(&value_bytes);
		payload.extend_from_slice(&[16u8; 16]);

		let result = wallet.deposit(payload.into());

		assert!(result.is_ok());

		let (deposit, remaining_payload) = result.expect("deposit failed");

		if let Deposit::Ether { sender, amount } = deposit {
			dbg!(sender.to_string());
			assert_eq!(sender, address);
			assert_eq!(amount, value);
		} else {
			panic!("invalid deposit type");
		}

		assert_eq!(wallet.balance_of(address), value);

		assert_eq!(remaining_payload, vec![16u8; 16]);
	}
}
