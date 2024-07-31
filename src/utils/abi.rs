pub mod encode {
	use crate::types::address::Address;
	use ethabi::{Function, Token, Uint};
	use std::error::Error;

	pub fn function_call(abi_json: &str, function_name: &str, params: Vec<Token>) -> Result<Vec<u8>, Box<dyn Error>> {
		let parsed_json = serde_json::from_str::<Vec<Function>>(abi_json)?;

		let func = parsed_json
			.iter()
			.find(|&f| f.name == function_name)
			.ok_or("Function not found in ABI")?;

		Ok(func.encode_input(&params)?)
	}

	pub mod ether {
		use super::*;

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

			function_call(abi_json, "withdrawEther", params)
		}
	}

	pub mod erc20 {
		use super::*;

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

			function_call(abi_json, "withdraw", params)
		}
	}

	pub mod erc721 {
		use super::*;

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

			function_call(abi_json, "safeTransferFrom", params)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::encode;
	use crate::{address, types::address::Address};
	use ethabi::{Token, Uint};

	#[test]
	fn test_ether_withdraw() {
		let address = address!("0x1234567890123456789012345678901234567890");
		let value = Uint::from(100);

		let encoded = encode::ether::withdraw(address, value).expect("encoding failed");
		let expected = hex::decode("522f681500000000000000000000000012345678901234567890123456789012345678900000000000000000000000000000000000000000000000000000000000000064").expect("decoding failed");

		assert_eq!(encoded, expected);
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

		let encoded = encode::function_call(abi_json, function_name, params).expect("encoding failed");
		let expected = hex::decode("a9059cbb000000000000000000000000f39fd6e51aad88f6f4ce6ab8827279cfffb9226600000000000000000000000000000000000000000000000000000000000003e8").expect("decoding failed");

		assert_eq!(encoded, expected);
	}
}
