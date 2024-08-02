pub mod abi {
	use crate::types::address::Address;
	use ethabi::{ParamType, Token, Uint};
	use std::error::Error;

	pub mod extract {
		use super::*;

		pub fn address(arg: &ethabi::Token) -> Result<Address, Box<dyn Error>> {
			arg.clone()
				.into_address()
				.map(Into::into)
				.ok_or_else(|| "invalid type for address".into())
		}

		pub fn uint(arg: &ethabi::Token) -> Result<Uint, Box<dyn Error>> {
			arg.clone()
				.into_uint()
				.map(Into::into)
				.ok_or_else(|| "invalid type for Uint".into())
		}

		pub fn array_of_uint(arg: &ethabi::Token) -> Result<Vec<Uint>, Box<dyn Error>> {
			arg.clone()
				.into_array()
				.ok_or_else(|| "invalid type for array of Uint".into())
				.and_then(|array| {
					array
						.into_iter()
						.map(|token| {
							token
								.into_uint()
								.ok_or_else(|| "invalid type for Uint".into())
								.map(Into::into)
						})
						.collect::<Result<Vec<Uint>, Box<dyn Error>>>()
				})
		}
	}

	pub mod utils {
		use super::*;

		pub fn size_of_packed_token(token: &Token) -> usize {
			match token {
				Token::Address(_) => 20,
				Token::FixedBytes(bytes) => bytes.len(),
				Token::Int(_) | Token::Uint(_) => 32,
				Token::String(_) | Token::Bytes(_) => 32,
				Token::Array(tokens) | Token::FixedArray(tokens) => tokens.iter().map(size_of_packed_token).sum(),
				Token::Tuple(tokens) => tokens.iter().map(size_of_packed_token).sum(),
				Token::Bool(_) => 1,
			}
		}

		pub fn size_of_packed_tokens(tokens: &[Token]) -> usize {
			tokens.iter().fold(0, |acc, token| acc + size_of_packed_token(token))
		}
	}

	pub mod encode {
		use super::*;
		use ethabi::Function;

		pub fn function_call(
			abi_json: &str,
			function_name: &str,
			params: Vec<Token>,
		) -> Result<Vec<u8>, Box<dyn Error>> {
			let parsed_json = serde_json::from_str::<Vec<Function>>(abi_json)?;

			let func = parsed_json
				.iter()
				.find(|&f| f.name == function_name)
				.ok_or("Function not found in ABI")?;

			Ok(func.encode_input(&params)?)
		}
	}

	pub mod decode {
		use utils::size_of_packed_token;

		use super::*;

		pub fn abi(params: &[ParamType], payload: &[u8]) -> Result<Vec<Token>, Box<dyn Error>> {
			let tokens = ethabi::decode(params, payload)?;
			Ok(tokens)
		}

		pub fn packed(params: &[ParamType], mut payload: &[u8]) -> Result<(Vec<Token>, Vec<u8>), Box<dyn Error>> {
			let mut tokens = Vec::new();

			for param in params {
				match param {
					ParamType::Address => {
						if payload.len() < 20 {
							return Err("Insufficient payload length for Address".into());
						}
						let address = Address::from_slice(&payload[..20]);
						tokens.push(Token::Address(address.into()));
						payload = &payload[20..];
					}
					ParamType::Uint(size) => {
						let byte_size = *size / 8;
						if payload.len() < byte_size {
							return Err(format!("Insufficient payload length for Uint of size {}", size).into());
						}
						let value = Uint::from_big_endian(&payload[..byte_size]);
						tokens.push(Token::Uint(value));
						payload = &payload[byte_size..];
					}
					ParamType::FixedBytes(size) => {
						if payload.len() < *size {
							return Err(format!("Insufficient payload length for FixedBytes of size {}", size).into());
						}
						let bytes = payload[..*size].to_vec();
						tokens.push(Token::FixedBytes(bytes));
						payload = &payload[*size..];
					}
					ParamType::Bytes => {
						if payload.len() < 32 {
							return Err("Insufficient payload length for Bytes size".into());
						}
						let size = Uint::from_big_endian(&payload[..32]).as_usize();
						if payload.len() < 32 + size {
							return Err("Insufficient payload length for Bytes".into());
						}
						let bytes = payload[32..32 + size].to_vec();
						tokens.push(Token::Bytes(bytes));
						payload = &payload[32 + size..];
					}
					ParamType::String => {
						if payload.len() < 32 {
							return Err("Insufficient payload length for String size".into());
						}
						let size = Uint::from_big_endian(&payload[..32]).as_usize();
						if payload.len() < 32 + size {
							return Err("Insufficient payload length for String".into());
						}
						let string = String::from_utf8(payload[32..32 + size].to_vec())?;
						tokens.push(Token::String(string));
						payload = &payload[32 + size..];
					}
					ParamType::Int(size) => {
						let byte_size = *size / 8;
						if payload.len() < byte_size {
							return Err(format!("Insufficient payload length for Int of size {}", size).into());
						}
						let value = Uint::from_big_endian(&payload[..byte_size]);
						tokens.push(Token::Int(value));
						payload = &payload[byte_size..];
					}
					ParamType::Bool => {
						if payload.is_empty() {
							return Err("Insufficient payload length for Bool".into());
						}
						let value = payload[0] != 0;
						tokens.push(Token::Bool(value));
						payload = &payload[1..];
					}
					ParamType::Array(param) => {
						if payload.len() < 32 {
							return Err("Insufficient payload length for Array size".into());
						}
						let size = Uint::from_big_endian(&payload[..32]).as_usize();
						payload = &payload[32..];
						let mut array = Vec::new();
						for _ in 0..size {
							let token = packed(&[*param.clone()], payload)?;
							array.push(token.0[0].clone());
							payload = &payload[size_of_packed_token(&token.0[0])..];
						}
						tokens.push(Token::Array(array));
					}
					ParamType::FixedArray(param, size) => {
						let mut array = Vec::new();
						for _ in 0..*size {
							let token = packed(&[*param.clone()], payload)?;
							array.push(token.0[0].clone());
							payload = &payload[size_of_packed_token(&token.0[0])..];
						}
						tokens.push(Token::FixedArray(array));
					}
					ParamType::Tuple(params) => {
						let mut tuple = Vec::new();
						for param in params {
							let token = packed(&[param.clone()], payload)?;
							tuple.push(token.0[0].clone());
							payload = &payload[size_of_packed_token(&token.0[0])..];
						}
						tokens.push(Token::Tuple(tuple));
					}
				}
			}

			Ok((tokens, payload.to_vec()))
		}
	}

	pub mod ether {
		use super::*;

		pub fn deposit(payload: Vec<u8>) -> Result<Vec<Token>, Box<dyn Error>> {
			let params = [ParamType::Address, ParamType::Uint(256)];

			decode::packed(&params, payload.as_ref()).map(|(tokens, _)| tokens)
		}

		pub fn withdraw(address: Address, value: Uint) -> Result<Vec<u8>, Box<dyn Error>> {
			let abi_json = r#"
			[
				{
					"name": "withdrawEther",
					"inputs": [
						{
							"internalType": "address",
							"name": "_receiver",
							"type": "address"
						},
						{
							"internalType": "uint256",
							"name": "_value",
							"type": "uint256"
						}
					],
					"outputs": [],
					"type": "function"
				}
			]"#;

			let params = vec![Token::Address(address.into()), Token::Uint(value)];

			encode::function_call(abi_json, "withdrawEther", params)
		}
	}

	pub mod erc20 {
		use super::*;

		pub fn deposit(payload: Vec<u8>) -> Result<Vec<Token>, Box<dyn Error>> {
			let params = [ParamType::Address, ParamType::Address, ParamType::Uint(256)];

			decode::packed(&params, payload.as_ref()).map(|(tokens, _)| tokens)
		}

		pub fn withdraw(address: Address, value: Uint) -> Result<Vec<u8>, Box<dyn Error>> {
			let abi_json = r#"
			[
				{
					"name": "transfer",
					"inputs": [
						{
							"internalType": "address",
							"name": "_receiver",
							"type": "address"
						},
						{
							"internalType": "uint256",
							"name": "_value",
							"type": "uint256"
						}
					],
					"outputs": [],
					"type": "function"
				}
			]"#;

			let params = vec![Token::Address(address.into()), Token::Uint(value)];

			encode::function_call(abi_json, "transfer", params)
		}
	}

	pub mod erc721 {
		use super::*;

		pub fn deposit(payload: Vec<u8>) -> Result<Vec<Token>, Box<dyn Error>> {
			let params = [ParamType::Address, ParamType::Address, ParamType::Uint(256)];

			decode::packed(&params, payload.as_ref()).map(|(tokens, _)| tokens)
		}

		pub fn withdraw(dapp_address: Address, address: Address, token_id: Uint) -> Result<Vec<u8>, Box<dyn Error>> {
			let abi_json = r#"
			[
				{
					"name": "safeTransferFrom",
					"inputs": [
						{
							"internalType": "address",
							"name": "_from",
							"type": "address"
						},
						{
							"internalType": "address",
							"name": "_to",
							"type": "address"
						},
						{
							"internalType": "uint256",
							"name": "_tokenId",
							"type": "uint256"
						}
					],
					"outputs": [],
					"type": "function"
				}
			]"#;

			let params = vec![
				Token::Address(dapp_address.into()),
				Token::Address(address.into()),
				Token::Uint(token_id),
			];

			encode::function_call(abi_json, "safeTransferFrom", params)
		}
	}

	pub mod erc1155 {
		use super::*;

		pub fn single_deposit(payload: Vec<u8>) -> Result<Vec<Token>, Box<dyn Error>> {
			let params = [
				ParamType::Address,
				ParamType::Address,
				ParamType::Uint(256),
				ParamType::Uint(256),
			];

			decode::packed(&params, payload.as_ref()).map(|(tokens, _)| tokens)
		}

		pub fn batch_deposit(payload: Vec<u8>) -> Result<Vec<Token>, Box<dyn Error>> {
			let params = [ParamType::Address, ParamType::Address];

			let (addresses_tokens, payload) = decode::packed(&params, payload.as_ref())?;

			let params = [
				ParamType::Array(Box::new(ParamType::Uint(256))),
				ParamType::Array(Box::new(ParamType::Uint(256))),
			];

			let values_tokens = decode::abi(&params, payload.as_ref())?;

			Ok([addresses_tokens, values_tokens].concat())
		}

		pub fn single_withdraw(
			dapp_address: Address,
			address: Address,
			token_id: Uint,
			amount: Uint,
			data: Vec<u8>,
		) -> Result<Vec<u8>, Box<dyn Error>> {
			let abi_json = r#"
			[
				{
					"name": "safeTransferFrom",
					"inputs": [
						{
							"internalType": "address",
							"name": "_from",
							"type": "address"
						},
						{
							"internalType": "address",
							"name": "_to",
							"type": "address"
						},
						{
							"internalType": "uint256",
							"name": "_id",
							"type": "uint256"
						},
						{
							"internalType": "uint256",
							"name": "_amount",
							"type": "uint256"
						},
						{
							"internalType": "bytes",
							"name": "_data",
							"type": "bytes"
						}
					],
					"outputs": [],
					"type": "function"
				}
			]"#;

			let params = vec![
				Token::Address(dapp_address.into()),
				Token::Address(address.into()),
				Token::Uint(token_id),
				Token::Uint(amount),
				Token::Bytes(data),
			];

			encode::function_call(abi_json, "safeTransferFrom", params)
		}

		pub fn batch_withdraw(
			dapp_address: Address,
			address: Address,
			withdrawals: Vec<(Uint, Uint)>,
			data: Vec<u8>,
		) -> Result<Vec<u8>, Box<dyn Error>> {
			let abi_json = r#"
			[
				{
					"name": "safeBatchTransferFrom",
					"inputs": [
						{
							"internalType": "address",
							"name": "_from",
							"type": "address"
						},
						{
							"internalType": "address",
							"name": "_to",
							"type": "address"
						},
						{
							"internalType": "uint256[]",
							"name": "_ids",
							"type": "uint256[]"
						},
						{
							"internalType": "uint256[]",
							"name": "_amounts",
							"type": "uint256[]"
						},
						{
							"internalType": "bytes",
							"name": "_data",
							"type": "bytes"
						}
					],
					"outputs": [],
					"type": "function"
				}
			]"#;

			let params = vec![
				Token::Address(dapp_address.into()),
				Token::Address(address.into()),
				Token::Array(
					withdrawals
						.iter()
						.map(|(id, _amount)| Token::Uint(id.clone()))
						.collect(),
				),
				Token::Array(
					withdrawals
						.iter()
						.map(|(_id, amount)| Token::Uint(amount.clone()))
						.collect(),
				),
				Token::Bytes(data),
			];

			encode::function_call(abi_json, "safeBatchTransferFrom", params)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::abi;
	use crate::{address, types::address::Address};
	use ethabi::{Token, Uint};

	#[test]
	fn test_ether_withdraw() {
		let address = address!("0x1234567890123456789012345678901234567890");
		let value = Uint::from(100);

		let encoded = abi::ether::withdraw(address, value).expect("encoding failed");
		let expected = hex::decode("522f681500000000000000000000000012345678901234567890123456789012345678900000000000000000000000000000000000000000000000000000000000000064").expect("decoding failed");

		assert_eq!(encoded, expected);
	}

	#[test]
	fn test_ether_deposit() {
		let payload = hex::decode(
			"f39fd6e51aad88f6f4ce6ab8827279cfffb922660000000000000000000000000000000000000000000000000000000000000064",
		)
		.expect("decoding failed");

		let tokens = abi::ether::deposit(payload).expect("decoding failed");

		assert_eq!(tokens.len(), 2);

		if let Token::Address(address) = &tokens[0] {
			assert_eq!(address, &address!("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"));
		} else {
			panic!("invalid token type");
		}

		if let Token::Uint(value) = &tokens[1] {
			assert_eq!(value, &Uint::from(100));
		} else {
			panic!("invalid token type");
		}
	}

	#[test]
	fn test_erc1155_batch_deposit() {
		let payload = hex::decode("f39fd6e51aad88f6f4ce6ab8827279cfffb92266f39fd6e51aad88f6f4ce6ab8827279cfffb92266000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000050000000000000000000000000000000000000000000000000000000000000006")
			.expect("decoding failed");

		let tokens = abi::erc1155::batch_deposit(payload).expect("decoding failed");
		assert_eq!(tokens.len(), 4);

		if let Token::Address(dapp_address) = &tokens[0] {
			assert_eq!(dapp_address, &address!("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"));
		} else {
			panic!("invalid token type");
		}

		if let Token::Address(address) = &tokens[1] {
			assert_eq!(address, &address!("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"));
		} else {
			panic!("invalid token type");
		}

		for i in 0..2 {
			if let Token::Array(array) = &tokens[i + 2] {
				assert_eq!(array.len(), 3);

				for (j, token) in array.iter().enumerate() {
					if let Token::Uint(value) = token {
						assert_eq!(value, &Uint::from((i * 3 + j + 1) as u128));
					} else {
						panic!("invalid token type");
					}
				}
			} else {
				panic!("invalid token type");
			}
		}
	}

	#[test]
	fn test_generic_encode_function_call() {
		let abi_json = r#"
		[
			{
				"name": "transfer",
				"inputs": [
					{
						"internalType": "address",
						"name": "to",
						"type": "address"
					},
					{
						"internalType": "uint256",
						"name": "value",
						"type": "uint256"
					}
				],
				"outputs": [],
				"type": "function"
			}
		]
		"#;

		let function_name = "transfer";
		let address = address!("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
		let value = Uint::from(1000);
		let params = vec![Token::Address(address.into()), Token::Uint(value)];

		let encoded = abi::encode::function_call(abi_json, function_name, params).expect("encoding failed");
		let expected = hex::decode("a9059cbb000000000000000000000000f39fd6e51aad88f6f4ce6ab8827279cfffb9226600000000000000000000000000000000000000000000000000000000000003e8").expect("decoding failed");

		assert_eq!(encoded, expected);
	}
}
