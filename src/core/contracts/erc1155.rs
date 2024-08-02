use crate::types::machine::Deposit;
use crate::utils::abi::abi;
use ethabi::{Address, Uint};
use std::collections::HashMap;
use std::error::Error;
use std::future::Future;

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

	pub fn transfer(
		&mut self,
		src_wallet: Address,
		dst_wallet: Address,
		token_address: Address,
		token_id: Uint,
		amount: Uint,
	) -> Result<(), Box<dyn Error>> {
		if src_wallet == dst_wallet {
			return Err("can't transfer to self".into());
		}

		let src_balance = self.balance_of(src_wallet, token_address, token_id);
		if src_balance < amount {
			return Err("insufficient funds".into());
		}

		let new_src_balance = src_balance - amount;
		let new_dst_balance = self.balance_of(dst_wallet, token_address, token_id) + amount;

		self.set_balance(src_wallet, token_address, token_id, new_src_balance);
		self.set_balance(dst_wallet, token_address, token_id, new_dst_balance);
		Ok(())
	}

	pub fn single_deposit(&mut self, payload: Vec<u8>) -> Result<(Deposit, Vec<u8>), Box<dyn Error>> {
		let args = abi::erc1155::single_deposit(payload)?;

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
			Deposit::ERC1155Single {
				sender: wallet_address,
				token: token_address,
				id: token_id,
				amount,
			},
			Vec::new(),
		))
	}

	pub fn batch_deposit(&mut self, payload: Vec<u8>) -> Result<Deposit, Box<dyn Error>> {
		let args = abi::erc1155::batch_deposit(payload)?;

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

		Ok(Deposit::ERC1155Batch {
			sender: wallet_address,
			token: token_address,
			ids: tokens_ids,
			amounts,
		})
	}

	pub fn deposit_payload(wallet_address: Address, token_address: Address, token_id: Uint, amount: Uint) -> Vec<u8> {
		let mut token_id_bytes = vec![0u8; 32];
		let mut amount_bytes = vec![0u8; 32];
		token_id.to_big_endian(&mut token_id_bytes);
		amount.to_big_endian(&mut amount_bytes);

		let mut payload = vec![0u8; 104];
		payload[0..20].copy_from_slice(wallet_address.as_ref());
		payload[20..40].copy_from_slice(token_address.as_ref());
		payload[40..72].copy_from_slice(&token_id_bytes);
		payload[72..104].copy_from_slice(&amount_bytes);

		payload
	}

	pub fn batch_deposit_payload(deposits: Vec<(Address, Address, Uint, Uint)>) -> Vec<u8> {
		let mut payload = Vec::new();

		for (wallet_address, token_address, token_id, amount) in deposits {
			let mut token_id_bytes = vec![0u8; 32];
			let mut amount_bytes = vec![0u8; 32];
			token_id.to_big_endian(&mut token_id_bytes);
			amount.to_big_endian(&mut amount_bytes);

			let mut deposit_payload = vec![0u8; 104];
			deposit_payload[0..20].copy_from_slice(wallet_address.as_ref());
			deposit_payload[20..40].copy_from_slice(token_address.as_ref());
			deposit_payload[40..72].copy_from_slice(&token_id_bytes);
			deposit_payload[72..104].copy_from_slice(&amount_bytes);

			payload.extend_from_slice(&deposit_payload);
		}

		payload
	}

	pub fn single_withdraw(
		&mut self,
		dapp_address: Address,
		wallet_address: Address,
		token_address: Address,
		token_id: Uint,
		amount: Uint,
		data: Vec<u8>,
	) -> Result<Vec<u8>, Box<dyn Error>> {
		let owner_balance = self.balance_of(wallet_address, token_address, token_id);
		if owner_balance < amount {
			return Err("insufficient funds".into());
		}

		let new_balance = owner_balance - amount;
		self.set_balance(wallet_address, token_address, token_id, new_balance);

		Ok(abi::erc1155::single_withdraw(
			dapp_address,
			wallet_address,
			token_id,
			amount,
			data,
		)?)
	}

	pub fn batch_withdraw(
		&mut self,
		dapp_address: Address,
		wallet_address: Address,
		token_address: Address,
		withdrawals: Vec<(Uint, Uint)>,
		data: Vec<u8>,
	) -> Result<Vec<u8>, Box<dyn Error>> {
		let mut changes: Vec<(Uint, Uint)> = Vec::new();
		for (token_id, amount) in &withdrawals {
			let owner_balance = self.balance_of(wallet_address, token_address, *token_id);
			if owner_balance < *amount {
				return Err("insufficient funds".into());
			}
			changes.push((*token_id, owner_balance - amount));
		}

		for (token_id, new_balance) in changes.clone() {
			self.set_balance(wallet_address, token_address, token_id, new_balance);
		}

		let result = abi::erc1155::batch_withdraw(dapp_address, wallet_address, withdrawals, data);

		match result {
			Ok(payload) => Ok(payload),
			Err(e) => {
				// Revert changes if the ABI call fails
				for (token_id, new_balance) in changes {
					let current_balance = self.balance_of(wallet_address, token_address, token_id);
					let amount = new_balance - current_balance; // Calculate the amount to revert
					self.set_balance(wallet_address, token_address, token_id, current_balance + amount);
				}
				Err(e)
			}
		}
	}
}

pub trait ERC1155Environment {
	fn erc1155_addresses(&self) -> impl Future<Output = Vec<Address>>;
	fn erc1155_withdraw(
		&self,
		wallet_address: Address,
		token_address: Address,
		token_id: Uint,
		amount: Uint,
	) -> impl Future<Output = Result<(), Box<dyn Error>>>;
	fn erc1155_batch_withdraw(
		&self,
		wallet_address: Address,
		withdrawals: Vec<(Address, Uint, Uint)>,
	) -> impl Future<Output = Result<(), Box<dyn Error>>>;
	fn erc1155_transfer(
		&self,
		src_wallet: Address,
		dst_wallet: Address,
		token_address: Address,
		token_id: Uint,
		amount: Uint,
	) -> impl Future<Output = Result<(), Box<dyn Error>>>;
	fn erc1155_balance(
		&self,
		wallet_address: Address,
		token_address: Address,
		token_id: Uint,
	) -> impl Future<Output = Uint>;
}
