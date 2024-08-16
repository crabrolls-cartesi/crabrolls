use crate::{
	core::contracts::{erc1155::ERC1155Wallet, erc20::ERC20Wallet, erc721::ERC721Wallet, ether::EtherWallet},
	utils::parsers::deserializers::*,
};
use ethabi::{Address, Uint};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone)]
pub struct Metadata {
	pub input_index: u64,
	#[serde(alias = "msg_sender")]
	pub sender: Address,
	pub block_number: u64,
	pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
#[serde(rename_all = "lowercase", tag = "status")]
pub enum FinishStatus {
	Accept,
	Reject,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Advance {
	pub metadata: Metadata,
	#[serde(deserialize_with = "deserialize_string_of_bytes")]
	pub payload: Vec<u8>,
}

#[derive(Deserialize, Debug)]
pub struct Inspect {
	#[serde(deserialize_with = "deserialize_string_of_bytes")]
	pub payload: Vec<u8>,
}

#[derive(Debug)]
pub enum Input {
	Advance(Advance),
	Inspect(Inspect),
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum Output {
	Voucher {
		destination: Address,
		#[serde(serialize_with = "serialize_bytes_as_string")]
		payload: Vec<u8>,
	},
	Notice {
		#[serde(serialize_with = "serialize_bytes_as_string")]
		payload: Vec<u8>,
	},
	Report {
		#[serde(serialize_with = "serialize_bytes_as_string")]
		payload: Vec<u8>,
	},
}

#[derive(Debug, Clone, PartialEq)]
pub enum Deposit {
	Ether {
		sender: Address,
		amount: Uint,
	},
	ERC20 {
		sender: Address,
		token: Address,
		amount: Uint,
	},
	ERC721 {
		sender: Address,
		token: Address,
		id: Uint,
	},
	ERC1155 {
		sender: Address,
		token: Address,
		ids_amounts: Vec<(Uint, Uint)>,
	},
}

impl TryFrom<Deposit> for Vec<u8> {
	type Error = Box<dyn std::error::Error>;

	fn try_from(deposit: Deposit) -> Result<Self, Self::Error> {
		match deposit {
			Deposit::Ether { sender, amount } => Ok(EtherWallet::deposit_payload(sender, amount)?),
			Deposit::ERC20 { sender, token, amount } => Ok(ERC20Wallet::deposit_payload(sender, token, amount)?),
			Deposit::ERC721 { sender, token, id } => Ok(ERC721Wallet::deposit_payload(sender, token, id)?),
			Deposit::ERC1155 {
				sender,
				token,
				ids_amounts,
			} => Ok(ERC1155Wallet::deposit_payload(sender, token, ids_amounts)?),
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub enum PortalHandlerConfig {
	Handle { advance: bool }, // Handle the portals and pass the payload/deposit to the app if advance is true
	Ignore,                   // Ignore the deposit handle and pass the payload to the app
	Dispense,                 // Dispense the deposit and discard the advance input
}

impl Default for PortalHandlerConfig {
	fn default() -> Self {
		Self::Handle { advance: true }
	}
}
