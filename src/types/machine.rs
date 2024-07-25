use crate::{types::address::Address, utils::parsers::deserializers::*};
use num_bigint::BigInt;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone)]
pub struct Metadata {
    pub input_index: u64,
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

#[derive(Deserialize, Debug)]
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
        amount: BigInt,
    },
    ERC20 {
        sender: Address,
        token: Address,
        amount: BigInt,
    },
    ERC721 {
        sender: Address,
        token: Address,
        id: BigInt,
    },
    ERC1155Single {
        sender: Address,
        token: Address,
        id: BigInt,
        amount: BigInt,
    },
    ERC1155Batch {
        sender: Address,
        token: Address,
        ids: Vec<BigInt>,
        amounts: Vec<BigInt>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Payload {
    input: Vec<u8>,
    deposit: Option<Deposit>,
}

impl Payload {
    pub fn new(input: Vec<u8>, deposit: Option<Deposit>) -> Self {
        Self { input, deposit }
    }

    pub fn get_input(&self) -> &[u8] {
        &self.input
    }

    pub fn get_deposit(&self) -> &Option<Deposit> {
        &self.deposit
    }

    pub fn is_deposit(&self) -> bool {
        self.deposit.is_some()
    }
}

impl From<Vec<u8>> for Payload {
    fn from(input: Vec<u8>) -> Self {
        Self::new(input, None)
    }
}

impl From<(Vec<u8>, Deposit)> for Payload {
    fn from((input, deposit): (Vec<u8>, Deposit)) -> Self {
        Self::new(input, Some(deposit))
    }
}
