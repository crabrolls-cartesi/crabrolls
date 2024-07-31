use ethabi::Address as EthAddress;
use serde::{Deserialize, Serialize};

#[macro_export]
macro_rules! address {
	($address:expr) => {
		(|| -> Address { $address.parse().expect("Invalid address format") })()
	};
}

#[repr(C)]
#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Default, Debug, Hash)]
pub struct H160(pub [u8; 20]);

impl H160 {
	pub fn new(inner: [u8; 20]) -> Self {
		H160(inner)
	}

	pub fn zero() -> Self {
		H160([0u8; 20])
	}

	pub fn is_zero(&self) -> bool {
		self.0.iter().all(|&x| x == 0)
	}

	pub fn as_bytes(&self) -> &[u8; 20] {
		&self.0
	}
}

impl std::fmt::Display for H160 {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "0x{}", hex::encode(&self.0))
	}
}

impl Serialize for H160 {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::ser::Serializer,
	{
		serializer.serialize_str(&format!("0x{}", hex::encode(&self.0)))
	}
}

impl<'de> Deserialize<'de> for H160 {
	fn deserialize<D>(deserializer: D) -> Result<H160, D::Error>
	where
		D: serde::de::Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		let bytes = hex::decode(&s[2..]).map_err(serde::de::Error::custom)?;
		let mut inner = [0u8; 20];
		inner.copy_from_slice(&bytes);
		Ok(H160(inner))
	}
}

impl std::str::FromStr for H160 {
	type Err = hex::FromHexError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let s = s.trim_start_matches("0x");
		let bytes = hex::decode(s)?;
		let mut inner = [0u8; 20];
		inner.copy_from_slice(&bytes);
		Ok(H160(inner))
	}
}

impl From<EthAddress> for H160 {
	fn from(address: EthAddress) -> Self {
		let mut inner = [0u8; 20];
		inner.copy_from_slice(&address.as_bytes());
		H160(inner)
	}
}

impl From<H160> for EthAddress {
	fn from(address: H160) -> Self {
		EthAddress::from_slice(&address.0)
	}
}

impl From<H160> for [u8; 20] {
	fn from(address: H160) -> Self {
		address.0
	}
}

impl AsRef<[u8; 20]> for H160 {
	fn as_ref(&self) -> &[u8; 20] {
		&self.0
	}
}

impl From<H160> for Vec<u8> {
	fn from(address: H160) -> Self {
		address.0.to_vec()
	}
}

impl From<H160> for String {
	fn from(address: H160) -> Self {
		address.to_string()
	}
}

impl From<&[u8]> for H160 {
	fn from(bytes: &[u8]) -> Self {
		let mut inner = [0u8; 20];
		inner.copy_from_slice(bytes);
		H160(inner)
	}
}

impl From<&str> for H160 {
	fn from(s: &str) -> Self {
		s.parse().expect("Invalid address format")
	}
}

impl TryFrom<Vec<u8>> for H160 {
	type Error = &'static str;

	fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
		if bytes.len() != 20 {
			Err("Invalid address length")
		} else {
			let mut inner = [0u8; 20];
			inner.copy_from_slice(&bytes);
			Ok(H160(inner))
		}
	}
}

pub type Address = H160;
