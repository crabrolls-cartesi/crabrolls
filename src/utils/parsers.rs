use ethers::utils::hex;
use serde::{Deserialize, Deserializer};

#[macro_export]
macro_rules! address {
    ($address:expr) => {
        (|| -> Address { $address.parse().expect("Invalid address format") })()
    };
}

pub fn deserialize_string_of_bytes<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    Ok(hex::decode(&s[2..]).map_err(serde::de::Error::custom)?)
}
