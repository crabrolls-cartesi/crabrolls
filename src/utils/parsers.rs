#[macro_export]
macro_rules! address {
    ($address:expr) => {
        (|| -> Address { $address.parse().expect("Invalid address format") })()
    };
}
