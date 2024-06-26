#[macro_export]
macro_rules! address {
    ($address:expr) => {
        (|| -> Address { $address.parse().expect("Invalid address format") })()
    };
}

pub mod deserializers {
    use serde::{Deserialize, Deserializer};

    pub fn deserialize_string_of_bytes<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        Ok(hex::decode(&s[2..]).map_err(serde::de::Error::custom)?)
    }

    pub fn serialize_bytes_as_string<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("0x{}", hex::encode(bytes)))
    }
}
