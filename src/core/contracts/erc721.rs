use crate::types::machine::Deposit;
use crate::utils::abi::abi;
use ethabi::{Address, Uint};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::future::Future;

pub struct ERC721Wallet {
	ownership: HashMap<Address, HashSet<(Address, Uint)>>,
}

impl ERC721Wallet {
	pub fn new() -> Self {
		ERC721Wallet {
			ownership: HashMap::new(),
		}
	}

	pub fn addresses(&self) -> Vec<Address> {
		let mut addresses: Vec<Address> = self.ownership.keys().cloned().collect();
		addresses.sort();
		addresses
	}

	pub fn add_token(&mut self, owner: Address, token_address: Address, token_id: Uint) {
		self.ownership
			.entry(owner)
			.or_insert_with(HashSet::new)
			.insert((token_address, token_id));
	}

	pub fn remove_token(&mut self, owner: Address, token_address: Address, token_id: Uint) {
		if let Some(tokens) = self.ownership.get_mut(&owner) {
			tokens.remove(&(token_address, token_id));
			if tokens.is_empty() {
				self.ownership.remove(&owner);
			}
		}
	}

	pub fn owner_of(&self, token_address: Address, token_id: Uint) -> Option<Address> {
		for (owner, tokens) in &self.ownership {
			if tokens.contains(&(token_address, token_id)) {
				return Some(owner.clone());
			}
		}
		None
	}

	pub fn transfer(
		&mut self,
		src_wallet: Address,
		dst_wallet: Address,
		token_address: Address,
		token_id: Uint,
	) -> Result<(), Box<dyn Error>> {
		if src_wallet == dst_wallet {
			return Err("can't transfer to self".into());
		}

		let owner = self.owner_of(token_address, token_id).ok_or("token not owned")?;
		if owner != src_wallet {
			return Err("source wallet does not own the token".into());
		}

		self.remove_token(src_wallet, token_address, token_id);
		self.add_token(dst_wallet, token_address, token_id);
		Ok(())
	}

	pub fn deposit(&mut self, payload: Vec<u8>) -> Result<(Deposit, Vec<u8>), Box<dyn Error>> {
		let args = abi::erc721::deposit(payload.clone())?;

		let token_address = abi::extract::address(&args[0])?;
		let wallet_address = abi::extract::address(&args[1])?;
		let token_id = abi::extract::uint(&args[2])?;

		debug!(
			"new erc721 deposit from {:?} with token {:?} and id {:?}",
			wallet_address, token_address, token_id
		);

		self.add_token(wallet_address, token_address, token_id);

		let deposit = Deposit::ERC721 {
			sender: wallet_address,
			token: token_address,
			id: token_id,
		};

		Ok((deposit, payload[abi::utils::size_of_packed_tokens(&args)..].to_vec()))
	}

	pub fn deposit_payload(
		wallet_address: Address,
		token_address: Address,
		token_id: Uint,
	) -> Result<Vec<u8>, Box<dyn Error>> {
		abi::erc721::deposit_payload(wallet_address, token_address, token_id)
	}

	pub fn withdraw(
		&mut self,
		dapp_address: Address,
		wallet_address: Address,
		token_address: Address,
		token_id: Uint,
	) -> Result<Vec<u8>, Box<dyn Error>> {
		let owner = self.owner_of(token_address, token_id).ok_or("token not owned")?;
		if owner != wallet_address {
			return Err("wallet does not own the token".into());
		}

		let result = abi::erc721::withdraw(dapp_address, wallet_address, token_id);

		match result {
			Ok(payload) => {
				self.remove_token(wallet_address, token_address, token_id);
				Ok(payload)
			}
			Err(e) => Err(e),
		}
	}
}

pub trait ERC721Environment {
	fn erc721_addresses(&self) -> impl Future<Output = Vec<Address>>;
	fn erc721_withdraw(
		&self,
		wallet_address: Address,
		token_address: Address,
		token_id: Uint,
	) -> impl Future<Output = Result<(), Box<dyn Error>>>;
	fn erc721_transfer(
		&self,
		source_wallet: Address,
		destination_wallet: Address,
		token_address: Address,
		token_id: Uint,
	) -> impl Future<Output = Result<(), Box<dyn Error>>>;
	fn erc721_owner_of(&self, token_address: Address, token_id: Uint) -> impl Future<Output = Option<Address>>;
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{address, uint};

	#[test]
	fn test_erc721_wallet_initialization() {
		let wallet = ERC721Wallet::new();
		assert!(wallet.ownership.is_empty());
	}

	#[test]
	fn test_add_remove_token() {
		let mut wallet = ERC721Wallet::new();
		let wallet_address = address!("0x0000000000000000000000000000000000000001");
		let token_address = address!("0x0000000000000000000000000000000000000002");

		wallet.add_token(wallet_address, token_address, uint!(1));
		assert_eq!(wallet.owner_of(token_address, uint!(1)), Some(wallet_address));

		wallet.remove_token(wallet_address, token_address, uint!(1));
		assert_eq!(wallet.owner_of(token_address, uint!(1)), None);
	}

	#[test]
	fn test_transfer() {
		let mut wallet = ERC721Wallet::new();
		let src_wallet = address!("0x0000000000000000000000000000000000000001");
		let dst_wallet = address!("0x0000000000000000000000000000000000000002");
		let token_address = address!("0x0000000000000000000000000000000000000003");

		wallet.add_token(src_wallet, token_address, uint!(1));
		let result = wallet.transfer(src_wallet, dst_wallet, token_address, uint!(1));
		assert!(result.is_ok());
		assert_eq!(wallet.owner_of(token_address, uint!(1)), Some(dst_wallet));
	}

	#[test]
	fn test_transfer_to_self() {
		let mut wallet = ERC721Wallet::new();
		let wallet_address = address!("0x0000000000000000000000000000000000000001");
		let token_address = address!("0x0000000000000000000000000000000000000002");

		wallet.add_token(wallet_address, token_address, uint!(1));
		let result = wallet.transfer(wallet_address, wallet_address, token_address, uint!(1));
		assert_eq!(result.unwrap_err().to_string(), "can't transfer to self");
	}

	#[test]
	fn test_deposit() {
		let mut wallet = ERC721Wallet::new();
		let wallet_address = address!("0x0000000000000000000000000000000000000001");
		let token_address = address!("0x0000000000000000000000000000000000000002");

		let token_id = uint!(uint!(1));
		let mut token_id_bytes = [0u8; 32];
		token_id.to_big_endian(&mut token_id_bytes);

		let mut payload = vec![0u8; 72];
		payload[0..20].copy_from_slice(token_address.as_bytes());
		payload[20..40].copy_from_slice(wallet_address.as_bytes());
		payload[40..72].copy_from_slice(&token_id_bytes);

		let result = wallet.deposit(payload.to_vec());
		assert!(result.is_ok());

		let (deposit, remaining_payload) = result.expect("deposit failed");

		if let Deposit::ERC721 {
			sender,
			token,
			id: token_id,
		} = deposit
		{
			assert_eq!(sender, wallet_address);
			assert_eq!(token, token_address);
			assert_eq!(token_id, uint!(1));
		} else {
			panic!("invalid deposit type");
		}

		assert_eq!(wallet.owner_of(token_address, uint!(1)), Some(wallet_address));
		assert!(remaining_payload.is_empty());
	}

	#[test]
	fn test_withdraw() {
		let mut wallet = ERC721Wallet::new();
		let wallet_address = address!("0x0000000000000000000000000000000000000001");
		let token_address = address!("0x0000000000000000000000000000000000000002");
		let dapp_address = address!("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266");

		wallet.add_token(wallet_address, token_address, uint!(1));
		let result = wallet.withdraw(dapp_address, wallet_address, token_address, uint!(1));
		assert!(result.is_ok());
		assert_eq!(wallet.owner_of(token_address, uint!(1)), None);
	}

	#[test]
	fn test_withdraw_not_owned() {
		let mut wallet = ERC721Wallet::new();
		let wallet_address = address!("0x0000000000000000000000000000000000000001");
		let token_address = address!("0x0000000000000000000000000000000000000002");
		let dapp_address = address!("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266");

		let result = wallet.withdraw(dapp_address, wallet_address, token_address, uint!(1));
		assert_eq!(result.unwrap_err().to_string(), "token not owned");
	}
}
