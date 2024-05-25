use serde::{Deserialize, Serialize};

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct H160(pub [u8; 20]);

impl std::fmt::Display for H160 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "0x{}", hex::encode(&self.0))
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

pub type Address = H160;
