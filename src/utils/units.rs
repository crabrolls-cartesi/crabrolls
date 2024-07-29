pub mod wei {
	use ethabi::Uint;

	pub fn to_ether(wei: Uint) -> f64 {
		wei.as_u128() as f64 / 1_000_000_000_000_000_000.0
	}

	pub fn from_ether(ether: f64) -> Uint {
		Uint::from((ether * 1_000_000_000_000_000_000.0) as u128)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use ethabi::Uint;

	#[test]
	fn test_to_ether() {
		let wei_value = Uint::from(1_000_000_000_000_000_000u64);
		let ether_value = wei::to_ether(wei_value);
		assert_eq!(ether_value, 1.0);

		let wei_value = Uint::from(2_000_000_000_000_000_000u64);
		let ether_value = wei::to_ether(wei_value);
		assert_eq!(ether_value, 2.0);
	}

	#[test]
	fn test_to_ether_small_value() {
		let wei_value = Uint::from(1_000_000_000_000_000u64); // 0.001 ether
		let ether_value = wei::to_ether(wei_value);
		assert_eq!(ether_value, 0.001);
	}

	#[test]
	fn test_to_ether_large_value() {
		let wei_value = Uint::from(1_000_000_000_000_000_000_000_000u128); // 1,000 ether
		let ether_value = wei::to_ether(wei_value);
		assert_eq!(ether_value, 1_000_000.0);
	}
}
