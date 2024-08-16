use ethabi::{Address, Uint};

#[macro_export]
macro_rules! address {
	($address:expr) => {
		(|| -> Address { $address.parse().expect("Invalid address format") })()
	};
}

#[macro_export]
macro_rules! uint {
	($value:expr) => {
		(|| -> Uint { Uint::from($value) })()
	};
}

pub use address;
pub use uint;

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_address_macro() {
		let address = address!("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266");

		assert_eq!(
			address,
			Address::from([
				0xf3, 0x9f, 0xd6, 0xe5, 0x1a, 0xad, 0x88, 0xf6, 0xf4, 0xce, 0x6a, 0xb8, 0x82, 0x72, 0x79, 0xcf, 0xff,
				0xb9, 0x22, 0x66
			])
		);
	}

	#[test]
	fn test_uint_macro() {
		let value = uint!(100);

		assert_eq!(value, Uint::from(100u64));
	}
}
