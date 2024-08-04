use crate::types::machine::Deposit;
use crate::utils::abi::abi;
use ethabi::{Address, Uint};
use std::collections::HashMap;
use std::error::Error;
use std::future::Future;

pub trait IntoIdsAmountsIter {
	fn into_inner_iter(self) -> Box<dyn Iterator<Item = (Uint, Uint)>>;
}

impl IntoIdsAmountsIter for (Uint, Uint) {
	fn into_inner_iter(self) -> Box<dyn Iterator<Item = (Uint, Uint)>> {
		Box::new(std::iter::once(self))
	}
}

impl IntoIdsAmountsIter for Vec<(Uint, Uint)> {
	fn into_inner_iter(self) -> Box<dyn Iterator<Item = (Uint, Uint)>> {
		Box::new(self.into_iter())
	}
}

pub trait IntoIdsIter {
	fn into_inner_iter(self) -> Box<dyn Iterator<Item = Uint>>;
}

impl IntoIdsIter for Uint {
	fn into_inner_iter(self) -> Box<dyn Iterator<Item = Uint>> {
		Box::new(std::iter::once(self))
	}
}

impl IntoIdsIter for Vec<Uint> {
	fn into_inner_iter(self) -> Box<dyn Iterator<Item = Uint>> {
		Box::new(self.into_iter())
	}
}

pub struct ERC1155Wallet {
	balances: HashMap<(Address, Address, Uint), Uint>,
}

impl ERC1155Wallet {
	pub fn new() -> Self {
		ERC1155Wallet {
			balances: HashMap::new(),
		}
	}

	pub fn addresses(&self) -> Vec<Address> {
		let mut addresses: Vec<Address> = self.balances.keys().map(|(a, _, _)| *a).collect();
		addresses.sort();
		addresses.dedup();
		addresses
	}

	pub fn set_balance(&mut self, owner: Address, token_address: Address, token_id: Uint, amount: Uint) {
		if amount.is_zero() {
			self.balances.remove(&(owner, token_address, token_id));
		} else {
			self.balances.insert((owner, token_address, token_id), amount);
		}
	}

	pub fn balance_of(&self, owner: Address, token_address: Address, token_id: Uint) -> Uint {
		self.balances
			.get(&(owner, token_address, token_id))
			.cloned()
			.unwrap_or_else(Uint::zero)
	}

	pub fn transfer<I>(
		&mut self,
		src_wallet: Address,
		dst_wallet: Address,
		token_address: Address,
		transfers: I,
	) -> Result<(), Box<dyn Error>>
	where
		I: IntoIdsAmountsIter,
	{
		if src_wallet == dst_wallet {
			return Err("can't transfer to self".into());
		}

		let transfers: Vec<(Uint, Uint)> = transfers.into_inner_iter().collect();

		for (token_id, amount) in &transfers {
			let src_balance = self.balance_of(src_wallet, token_address, *token_id);
			if src_balance < *amount {
				return Err("insufficient funds".into());
			}
		}

		for (token_id, amount) in &transfers {
			let src_balance = self.balance_of(src_wallet, token_address, *token_id);
			let dst_balance = self.balance_of(dst_wallet, token_address, *token_id);

			self.set_balance(src_wallet, token_address, *token_id, src_balance - *amount);
			self.set_balance(dst_wallet, token_address, *token_id, dst_balance + *amount);
		}

		Ok(())
	}

	pub fn single_deposit(&mut self, payload: Vec<u8>) -> Result<(Deposit, Vec<u8>), Box<dyn Error>> {
		let args = abi::erc1155::single_deposit(payload.clone())?;

		let token_address = abi::extract::address(&args[0])?;
		let wallet_address = abi::extract::address(&args[1])?;
		let token_id = abi::extract::uint(&args[2])?;
		let amount = abi::extract::uint(&args[3])?;

		debug!(
			"new ERC1155 single deposit from {:?} with value {:?}",
			wallet_address, amount
		);

		let new_balance = self.balance_of(wallet_address, token_address, token_id) + amount;
		self.set_balance(wallet_address, token_address, token_id, new_balance);

		Ok((
			Deposit::ERC1155 {
				sender: wallet_address,
				token: token_address,
				ids_amounts: vec![(token_id, amount)],
			},
			payload[abi::utils::size_of_packed_tokens(&args)..].to_vec(),
		))
	}

	pub fn batch_deposit(&mut self, payload: Vec<u8>) -> Result<(Deposit, Vec<u8>), Box<dyn Error>> {
		let args = abi::erc1155::batch_deposit(payload.clone())?;

		let token_address = abi::extract::address(&args[0])?;
		let wallet_address = abi::extract::address(&args[1])?;
		let tokens_ids = abi::extract::array_of_uint(&args[2])?;
		let amounts = abi::extract::array_of_uint(&args[3])?;

		debug!(
			"new ERC1155 batch deposit from {:?} with values {:?}",
			wallet_address, amounts
		);

		for (token_id, amount) in tokens_ids.iter().zip(amounts.iter()) {
			let new_balance = self.balance_of(wallet_address, token_address, *token_id) + *amount;
			self.set_balance(wallet_address, token_address, *token_id, new_balance);
		}

		Ok((
			Deposit::ERC1155 {
				sender: wallet_address,
				token: token_address,
				ids_amounts: tokens_ids.iter().cloned().zip(amounts.iter().cloned()).collect(),
			},
			payload[abi::utils::size_of_packed_tokens(&args)..].to_vec(),
		))
	}

	pub fn deposit_payload<I>(
		wallet_address: Address,
		token_address: Address,
		deposits: I,
	) -> Result<Vec<u8>, Box<dyn Error>>
	where
		I: IntoIdsAmountsIter,
	{
		let deposits: Vec<(Uint, Uint)> = deposits.into_inner_iter().collect();
		match deposits.len() {
			1 => {
				let (token_id, amount) = deposits.into_iter().next().unwrap();
				abi::erc1155::single_deposit_payload(wallet_address, token_address, token_id, amount)
			}
			_ => abi::erc1155::batch_deposit_payload(wallet_address, token_address, deposits.into_iter().collect()),
		}
	}

	pub fn withdraw<I>(
		&mut self,
		dapp_address: Address,
		wallet_address: Address,
		token_address: Address,
		withdrawals: I,
		data: Option<Vec<u8>>,
	) -> Result<Vec<u8>, Box<dyn Error>>
	where
		I: IntoIdsAmountsIter,
	{
		let mut changes: Vec<(Uint, Uint)> = Vec::new();
		let withdrawals: Vec<(Uint, Uint)> = withdrawals.into_inner_iter().collect();
		for (token_id, amount) in &withdrawals {
			let owner_balance = self.balance_of(wallet_address, token_address, *token_id);
			if owner_balance < *amount {
				return Err("insufficient funds".into());
			}
			changes.push((*token_id, owner_balance - amount));
		}

		let result = abi::erc1155::batch_withdraw(dapp_address, wallet_address, withdrawals, data.unwrap_or_default());

		match result {
			Ok(payload) => {
				for (token_id, new_balance) in changes {
					self.set_balance(wallet_address, token_address, token_id, new_balance);
				}
				Ok(payload)
			}
			Err(e) => Err(e),
		}
	}
}

pub trait ERC1155Environment {
	fn erc1155_addresses(&self) -> impl Future<Output = Vec<Address>>;
	fn erc1155_withdraw<I>(
		&self,
		wallet_address: Address,
		token_address: Address,
		withdrawals: I,
		data: Option<Vec<u8>>,
	) -> impl Future<Output = Result<(), Box<dyn Error>>>
	where
		I: IntoIdsAmountsIter;
	fn erc1155_transfer<I>(
		&self,
		src_wallet: Address,
		dst_wallet: Address,
		token_address: Address,
		transfers: I,
	) -> impl Future<Output = Result<(), Box<dyn Error>>>
	where
		I: IntoIdsAmountsIter;
	fn erc1155_balance(
		&self,
		wallet_address: Address,
		token_address: Address,
		token_id: Uint,
	) -> impl Future<Output = Uint>;
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_addresses() {
		let mut wallet = ERC1155Wallet::new();
		let address1 = Address::from_low_u64_be(1);
		let address2 = Address::from_low_u64_be(2);
		let token_address = Address::from_low_u64_be(3);
		let token_id = Uint::from(1);
		let amount = Uint::from(100);

		wallet.set_balance(address1, token_address, token_id, amount);
		wallet.set_balance(address2, token_address, token_id, amount);

		let addresses = wallet.addresses();
		assert_eq!(addresses.len(), 2);
		assert!(addresses.contains(&address1));
		assert!(addresses.contains(&address2));
	}

	#[test]
	fn test_set_balance() {
		let mut wallet = ERC1155Wallet::new();
		let owner = Address::from_low_u64_be(1);
		let token_address = Address::from_low_u64_be(2);
		let token_id = Uint::from(1);
		let amount = Uint::from(100);

		wallet.set_balance(owner, token_address, token_id, amount);
		assert_eq!(wallet.balance_of(owner, token_address, token_id), amount);

		wallet.set_balance(owner, token_address, token_id, Uint::zero());
		assert_eq!(wallet.balance_of(owner, token_address, token_id), Uint::zero());
	}

	#[test]
	fn test_transfer() {
		let mut wallet = ERC1155Wallet::new();
		let src_wallet = Address::from_low_u64_be(1);
		let dst_wallet = Address::from_low_u64_be(2);
		let token_address = Address::from_low_u64_be(3);
		let token_id = Uint::from(1);
		let amount = Uint::from(100);

		wallet.set_balance(src_wallet, token_address, token_id, amount);

		let transfer_amount = Uint::from(50);
		assert!(wallet
			.transfer(src_wallet, dst_wallet, token_address, vec![(token_id, transfer_amount)])
			.is_ok());
		assert_eq!(wallet.balance_of(src_wallet, token_address, token_id), transfer_amount);
		assert_eq!(wallet.balance_of(dst_wallet, token_address, token_id), transfer_amount);

		assert!(wallet
			.transfer(
				src_wallet,
				dst_wallet,
				token_address,
				vec![(token_id, transfer_amount + Uint::from(1))]
			)
			.is_err());
	}

	#[test]
	fn test_single_deposit() {
		let mut wallet = ERC1155Wallet::new();
		let token_address = Address::from_low_u64_be(1);
		let wallet_address = Address::from_low_u64_be(2);
		let token_id = Uint::from(1);
		let amount = Uint::from(100);

		let payload =
			ERC1155Wallet::deposit_payload(wallet_address, token_address, (token_id, amount)).expect("deposit payload");
		assert!(wallet.single_deposit(payload).is_ok());
		assert_eq!(wallet.balance_of(wallet_address, token_address, token_id), amount);
	}

	#[test]
	fn test_batch_deposit() {
		let mut wallet = ERC1155Wallet::new();
		let token_address = Address::from_low_u64_be(1);
		let wallet_address = Address::from_low_u64_be(2);
		let deposits = vec![(Uint::from(1), Uint::from(50)), (Uint::from(2), Uint::from(100))];

		let payload = ERC1155Wallet::deposit_payload(wallet_address, token_address, deposits.clone())
			.expect("batch deposit payload");
		assert!(wallet.batch_deposit(payload).is_ok());

		for (id, amount) in deposits {
			assert_eq!(wallet.balance_of(wallet_address, token_address, id), amount);
		}
	}

	#[test]
	fn test_single_withdraw() {
		let mut wallet = ERC1155Wallet::new();
		let dapp_address = Address::from_low_u64_be(1);
		let wallet_address = Address::from_low_u64_be(2);
		let token_address = Address::from_low_u64_be(3);
		let token_id = Uint::from(1);
		let amount = Uint::from(100);

		wallet.set_balance(wallet_address, token_address, token_id, amount);

		let withdraw_amount = Uint::from(50);

		assert!(wallet
			.withdraw(
				dapp_address,
				wallet_address,
				token_address,
				(token_id, withdraw_amount),
				None
			)
			.is_ok());
		assert_eq!(
			wallet.balance_of(wallet_address, token_address, token_id),
			withdraw_amount
		);

		assert!(wallet
			.withdraw(
				dapp_address,
				wallet_address,
				token_address,
				(token_id, withdraw_amount + Uint::from(1)),
				None
			)
			.is_err());
	}

	#[test]
	fn test_batch_withdraw() {
		let mut wallet = ERC1155Wallet::new();
		let dapp_address = Address::from_low_u64_be(1);
		let wallet_address = Address::from_low_u64_be(2);
		let token_address = Address::from_low_u64_be(3);
		let withdrawals = vec![(Uint::from(1), Uint::from(50)), (Uint::from(2), Uint::from(100))];

		wallet.set_balance(wallet_address, token_address, Uint::from(1), Uint::from(100));
		wallet.set_balance(wallet_address, token_address, Uint::from(2), Uint::from(200));

		assert!(wallet
			.withdraw(dapp_address, wallet_address, token_address, withdrawals.clone(), None)
			.is_ok());

		for (token_id, amount) in withdrawals {
			assert_eq!(wallet.balance_of(wallet_address, token_address, token_id), amount);
		}

		let failing_withdrawals = vec![(Uint::from(1), Uint::from(100)), (Uint::from(2), Uint::from(200))];
		assert!(wallet
			.withdraw(dapp_address, wallet_address, token_address, failing_withdrawals, None)
			.is_err());
	}
}
