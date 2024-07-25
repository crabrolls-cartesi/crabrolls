pub mod encode {
    use crate::types::address::Address;
    use ethabi::{Function, Param, Token, Uint};
    use std::error::Error;

    pub fn ether_withdraw(address: Address, value: Uint) -> Result<Vec<u8>, Box<dyn Error>> {
        let function = Function {
            name: "withdrawEther".to_string(),
            inputs: vec![
                Param {
                    name: "address".to_string(),
                    kind: ethabi::ParamType::Address,
                    internal_type: None,
                },
                Param {
                    name: "value".to_string(),
                    kind: ethabi::ParamType::Uint(256),
                    internal_type: None,
                },
            ],
            outputs: vec![],
            state_mutability: ethabi::StateMutability::Payable,
            constant: None,
        };

        let tokens: Vec<Token> = vec![Token::Address(address.into()), Token::Uint(value)];

        Ok(function.encode_input(&tokens)?)
    }
}
