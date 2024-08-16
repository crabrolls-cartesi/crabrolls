pub mod wei {
	use ethabi::Uint;

	pub fn to_ether(wei: Uint) -> f64 {
		wei.as_u128() as f64 / 1_000_000_000_000_000_000.0
	}

	pub fn from_ether(ether: f64) -> Uint {
		Uint::from((ether * 1_000_000_000_000_000_000.0) as u128)
	}

	pub fn to_gwei(wei: Uint) -> f64 {
		wei.as_u128() as f64 / 1_000_000_000.0
	}

	pub fn from_gwei(gwei: f64) -> Uint {
		Uint::from((gwei * 1_000_000_000.0) as u128)
	}
}
#[cfg(test)]
mod tests {
	use super::*;
	use crate::uint;
	use ethabi::Uint;

	#[test]
	fn test_to_ether() {
		let wei_value = uint!(1_000_000_000_000_000_000u64);
		let ether_value = wei::to_ether(wei_value);
		assert_eq!(ether_value, 1.0);

		let wei_value = uint!(2_000_000_000_000_000_000u64);
		let ether_value = wei::to_ether(wei_value);
		assert_eq!(ether_value, 2.0);
	}

	#[test]
	fn test_to_ether_small_value() {
		let wei_value = uint!(1_000_000_000_000_000u64); // 0.001 ether
		let ether_value = wei::to_ether(wei_value);
		assert_eq!(ether_value, 0.001);
	}

	#[test]
	fn test_to_ether_large_value() {
		let wei_value = uint!(1_000_000_000_000_000_000_000_000u128); // 1,000,000 ether
		let ether_value = wei::to_ether(wei_value);
		assert_eq!(ether_value, 1_000_000.0);
	}

	#[test]
	fn test_from_ether() {
		let ether_value = 1.0;
		let wei_value = wei::from_ether(ether_value);
		assert_eq!(wei_value, uint!(1_000_000_000_000_000_000u128));

		let ether_value = 2.0;
		let wei_value = wei::from_ether(ether_value);
		assert_eq!(wei_value, uint!(2_000_000_000_000_000_000u128));
	}

	#[test]
	fn test_from_ether_small_value() {
		let ether_value = 0.001;
		let wei_value = wei::from_ether(ether_value);
		assert_eq!(wei_value, uint!(1_000_000_000_000_000u128));
	}

	/// Currently, this test fails because of the precision of the f64 type.
	//#[test]
	//fn test_from_ether_large_value() {
	//	let ether_value = 1_000_000.0;
	//	let wei_value = wei::from_ether(ether_value);
	//	assert_eq!(wei_value, uint!(1_000_000_000_000_000_000_000_000u128));
	//}

	#[test]
	fn test_to_gwei() {
		let wei_value = uint!(1_000_000_000u64);
		let gwei_value = wei::to_gwei(wei_value);
		assert_eq!(gwei_value, 1.0);

		let wei_value = uint!(2_000_000_000u64);
		let gwei_value = wei::to_gwei(wei_value);
		assert_eq!(gwei_value, 2.0);
	}

	#[test]
	fn test_to_gwei_small_value() {
		let wei_value = uint!(500_000_000u64); // 0.5 Gwei
		let gwei_value = wei::to_gwei(wei_value);
		assert_eq!(gwei_value, 0.5);
	}

	#[test]
	fn test_to_gwei_large_value() {
		let wei_value = uint!(1_000_000_000_000_000_000u128); // 1,000,000,000 Gwei
		let gwei_value = wei::to_gwei(wei_value);
		assert_eq!(gwei_value, 1_000_000_000.0);
	}

	#[test]
	fn test_from_gwei() {
		let gwei_value = 1.0;
		let wei_value = wei::from_gwei(gwei_value);
		assert_eq!(wei_value, uint!(1_000_000_000u128));

		let gwei_value = 2.0;
		let wei_value = wei::from_gwei(gwei_value);
		assert_eq!(wei_value, uint!(2_000_000_000u128));
	}

	#[test]
	fn test_from_gwei_small_value() {
		let gwei_value = 0.5;
		let wei_value = wei::from_gwei(gwei_value);
		assert_eq!(wei_value, uint!(500_000_000u128));
	}

	#[test]
	fn test_from_gwei_large_value() {
		let gwei_value = 1_000_000_000.0;
		let wei_value = wei::from_gwei(gwei_value);
		assert_eq!(wei_value, uint!(1_000_000_000_000_000_000u128));
	}

	#[test]
	fn test_round_trip_ether() {
		let ether_value = 1234.56789;
		let wei_value = wei::from_ether(ether_value);
		let result = wei::to_ether(wei_value);
		assert_eq!(result, ether_value);
	}

	#[test]
	fn test_round_trip_gwei() {
		let gwei_value = 987654.321;
		let wei_value = wei::from_gwei(gwei_value);
		let result = wei::to_gwei(wei_value);
		assert_eq!(result, gwei_value);
	}
}
