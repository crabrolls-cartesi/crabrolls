pub mod abi {
	use ethabi::{Address, ParamType, Token, Uint};
	use std::error::Error;

	pub mod extract {
		use super::*;

		pub fn address(arg: &ethabi::Token) -> Result<Address, Box<dyn Error>> {
			arg.clone()
				.into_address()
				.ok_or_else(|| "invalid type for address".into())
		}

		pub fn uint(arg: &ethabi::Token) -> Result<Uint, Box<dyn Error>> {
			arg.clone().into_uint().ok_or_else(|| "invalid type for Uint".into())
		}

		pub fn bool(arg: &ethabi::Token) -> Result<bool, Box<dyn Error>> {
			arg.clone().into_bool().ok_or_else(|| "invalid type for bool".into())
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
		use ethabi::{encode, Function, Token};
		use serde_json::from_str;
		use std::error::Error;

		pub fn function_call(
			abi_json: &str,
			function_name: &str,
			params: Vec<Token>,
		) -> Result<Vec<u8>, Box<dyn Error>> {
			let parsed_json: Vec<Function> = from_str(abi_json)?;
			let func = parsed_json
				.iter()
				.find(|&f| f.name == function_name)
				.ok_or("Function not found in ABI")?;
			Ok(func.encode_input(&params)?)
		}

		pub fn abi(tokens: &[Token]) -> Result<Vec<u8>, Box<dyn Error>> {
			Ok(encode(tokens))
		}

		pub fn pack(tokens: &[Token]) -> Result<Vec<u8>, Box<dyn Error>> {
			let mut payload = Vec::new();

			for token in tokens {
				match token {
					Token::Address(address) => payload.extend_from_slice(&address.as_bytes()[..]),
					Token::Uint(value) | Token::Int(value) => {
						let mut buf = [0u8; 32];
						value.to_big_endian(&mut buf);
						payload.extend_from_slice(&buf);
					}
					Token::FixedBytes(bytes) => payload.extend_from_slice(bytes),
					Token::Bool(value) => payload.push(if *value { 1 } else { 0 }),
					Token::String(string) => {
						let string_bytes = string.as_bytes();
						let size = string_bytes.len();
						let size_token = Token::Uint(size.into());
						let mut size_buf = encode(&[size_token]);
						size_buf.truncate(32);
						payload.extend_from_slice(&size_buf);
						payload.extend_from_slice(string_bytes);
					}
					Token::Bytes(bytes) => {
						let size = bytes.len();
						let size_token = Token::Uint(size.into());
						let mut size_buf = encode(&[size_token]);
						size_buf.truncate(32);
						payload.extend_from_slice(&size_buf);
						payload.extend_from_slice(bytes);
					}
					Token::Array(array) | Token::FixedArray(array) | Token::Tuple(array) => {
						for token in array {
							payload.extend_from_slice(&pack(&[token.clone()])?);
						}
					}
				}
			}

			Ok(payload)
		}
	}

	pub mod decode {
		use ethabi::{decode, ParamType, Token};
		use std::error::Error;

		use super::*;

		pub fn abi(params: &[ParamType], payload: &[u8]) -> Result<Vec<Token>, Box<dyn Error>> {
			Ok(decode(params, payload)?)
		}

		pub fn pack<'a>(
			params: &'a [ParamType],
			mut payload: &'a [u8],
		) -> Result<(Vec<Token>, Vec<u8>), Box<dyn Error>> {
			let mut tokens = Vec::new();

			for param in params {
				match param {
					ParamType::Address => {
						ensure_payload_length(&payload, 20, "Address")?;
						tokens.push(Token::Address(Address::from_slice(&payload[..20])));
						payload = &payload[20..];
					}
					ParamType::Uint(size) | ParamType::Int(size) => {
						let byte_size = size / 8;
						ensure_payload_length(&payload, byte_size, &format!("Uint/Int of size {}", size))?;
						tokens.push(Token::Uint(payload[..byte_size].into()));
						payload = &payload[byte_size..];
					}
					ParamType::FixedBytes(size) => {
						ensure_payload_length(&payload, *size, &format!("FixedBytes of size {}", size))?;
						tokens.push(Token::FixedBytes(payload[..*size].to_vec()));
						payload = &payload[*size..];
					}
					ParamType::Bytes | ParamType::String => {
						ensure_payload_length(&payload, 32, "Bytes/String size")?;
						let size = Uint::from(&payload[..32]).as_usize();
						ensure_payload_length(&payload, 32 + size, "Bytes/String")?;
						if let ParamType::Bytes = param {
							tokens.push(Token::Bytes(payload[32..32 + size].to_vec()));
						} else {
							tokens.push(Token::String(String::from_utf8(payload[32..32 + size].to_vec())?));
						}
						payload = &payload[32 + size..];
					}
					ParamType::Bool => {
						ensure_payload_length(&payload, 1, "Bool")?;
						tokens.push(Token::Bool(payload[0] != 0));
						payload = &payload[1..];
					}
					ParamType::Array(param) => {
						ensure_payload_length(&payload, 32, "Array size")?;
						let size = Uint::from(&payload[..32]).as_usize();
						payload = &payload[32..];
						let array = parse_array(param, size, payload)?;
						tokens.push(Token::Array(array.0));
						payload = array.1;
					}
					ParamType::FixedArray(param, size) => {
						let array = parse_fixed_array(param, *size, payload)?;
						tokens.push(Token::FixedArray(array.0));
						payload = array.1;
					}
					ParamType::Tuple(params) => {
						let tuple = parse_tuple(params, payload)?;
						tokens.push(Token::Tuple(tuple.0));
						payload = tuple.1;
					}
				}
			}

			Ok((tokens, payload.to_vec()))
		}

		fn ensure_payload_length(payload: &[u8], required_len: usize, type_desc: &str) -> Result<(), Box<dyn Error>> {
			if payload.len() < required_len {
				Err(format!("Insufficient payload length for {}", type_desc).into())
			} else {
				Ok(())
			}
		}

		fn parse_array<'a>(
			param: &'a ParamType,
			size: usize,
			mut payload: &'a [u8],
		) -> Result<(Vec<Token>, &'a [u8]), Box<dyn Error>> {
			let mut array = Vec::new();
			for _ in 0..size {
				let token = pack(&[param.clone()], payload)?;
				array.push(token.0[0].clone());
				payload = &payload[utils::size_of_packed_token(&token.0[0])..];
			}
			Ok((array, payload))
		}

		fn parse_fixed_array<'a>(
			param: &'a ParamType,
			size: usize,
			mut payload: &'a [u8],
		) -> Result<(Vec<Token>, &'a [u8]), Box<dyn Error>> {
			let mut array = Vec::new();
			for _ in 0..size {
				let token = pack(&[param.clone()], payload)?;
				array.push(token.0[0].clone());
				payload = &payload[utils::size_of_packed_token(&token.0[0])..];
			}
			Ok((array, payload))
		}

		fn parse_tuple<'a>(
			params: &'a [ParamType],
			mut payload: &'a [u8],
		) -> Result<(Vec<Token>, &'a [u8]), Box<dyn Error>> {
			let mut tuple = Vec::new();
			for param in params {
				let token = pack(&[param.clone()], payload)?;
				tuple.push(token.0[0].clone());
				payload = &payload[utils::size_of_packed_token(&token.0[0])..];
			}
			Ok((tuple, payload))
		}
	}

	pub mod ether {
		use super::*;

		pub fn deposit(payload: Vec<u8>) -> Result<Vec<Token>, Box<dyn Error>> {
			let params = [ParamType::Address, ParamType::Uint(256)];

			decode::pack(&params, payload.as_ref()).map(|(tokens, _)| tokens)
		}

		pub fn deposit_payload(address: Address, value: Uint) -> Result<Vec<u8>, Box<dyn Error>> {
			let tokens = vec![Token::Address(address), Token::Uint(value)];

			encode::pack(&tokens)
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

			let params = vec![Token::Address(address), Token::Uint(value)];

			encode::function_call(abi_json, "withdrawEther", params)
		}
	}

	pub mod erc20 {
		use super::*;

		pub fn deposit(payload: Vec<u8>) -> Result<Vec<Token>, Box<dyn Error>> {
			let params = [
				ParamType::Bool,
				ParamType::Address,
				ParamType::Address,
				ParamType::Uint(256),
			];

			decode::pack(&params, payload.as_ref()).map(|(tokens, _)| tokens)
		}

		pub fn deposit_payload(
			wallet_address: Address,
			token_address: Address,
			value: Uint,
		) -> Result<Vec<u8>, Box<dyn Error>> {
			let tokens = vec![
				Token::Address(token_address),
				Token::Address(wallet_address),
				Token::Uint(value),
			];

			encode::pack(&tokens)
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

			let params = vec![Token::Address(address), Token::Uint(value)];

			encode::function_call(abi_json, "transfer", params)
		}
	}

	pub mod erc721 {
		use super::*;

		pub fn deposit(payload: Vec<u8>) -> Result<Vec<Token>, Box<dyn Error>> {
			let params = [ParamType::Address, ParamType::Address, ParamType::Uint(256)];

			decode::pack(&params, payload.as_ref()).map(|(tokens, _)| tokens)
		}

		pub fn deposit_payload(
			wallet_address: Address,
			token_address: Address,
			token_id: Uint,
		) -> Result<Vec<u8>, Box<dyn Error>> {
			let tokens = vec![
				Token::Address(token_address),
				Token::Address(wallet_address),
				Token::Uint(token_id),
			];

			encode::pack(&tokens)
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
				Token::Address(dapp_address),
				Token::Address(address),
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

			decode::pack(&params, payload.as_ref()).map(|(tokens, _)| tokens)
		}

		pub fn batch_deposit(payload: Vec<u8>) -> Result<Vec<Token>, Box<dyn Error>> {
			let params = [ParamType::Address, ParamType::Address];

			let (addresses_tokens, payload) = decode::pack(&params, payload.as_ref())?;

			let params = [
				ParamType::Array(Box::new(ParamType::Uint(256))),
				ParamType::Array(Box::new(ParamType::Uint(256))),
			];

			let values_tokens = decode::abi(&params, payload.as_ref())?;

			Ok([addresses_tokens, values_tokens].concat())
		}

		pub fn single_deposit_payload(
			wallet_address: Address,
			token_address: Address,
			token_id: Uint,
			amount: Uint,
		) -> Result<Vec<u8>, Box<dyn Error>> {
			let tokens = vec![
				Token::Address(token_address),
				Token::Address(wallet_address),
				Token::Uint(token_id),
				Token::Uint(amount),
			];

			encode::pack(&tokens)
		}

		pub fn batch_deposit_payload(
			wallet_address: Address,
			token_address: Address,
			ids_amounts: Vec<(Uint, Uint)>,
		) -> Result<Vec<u8>, Box<dyn Error>> {
			let ids = ids_amounts.iter().map(|(id, _)| Token::Uint(id.clone())).collect();
			let amounts = ids_amounts
				.iter()
				.map(|(_, amount)| ethabi::Token::Uint(amount.clone()))
				.collect();

			let ids_amounts_bytes = encode::abi(&[Token::Array(ids), Token::Array(amounts)])?;

			let tokens = vec![Token::Address(token_address), Token::Address(wallet_address)];

			Ok(encode::pack(&tokens)?
				.iter()
				.chain(ids_amounts_bytes.iter())
				.cloned()
				.collect())
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
				Token::Address(dapp_address),
				Token::Address(address),
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
				Token::Address(dapp_address),
				Token::Address(address),
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
	use crate::address;
	use ethabi::{Address, Token, Uint};

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
		let params = vec![Token::Address(address), Token::Uint(value)];

		let encoded = abi::encode::function_call(abi_json, function_name, params).expect("encoding failed");
		let expected = hex::decode("a9059cbb000000000000000000000000f39fd6e51aad88f6f4ce6ab8827279cfffb9226600000000000000000000000000000000000000000000000000000000000003e8").expect("decoding failed");

		assert_eq!(encoded, expected);
	}
}
