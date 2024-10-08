extern crate pretty_env_logger;
#[macro_use]
extern crate log;

mod core;
mod types;
mod utils;

use core::{application::Application, environment::Environment};
use types::machine::{FinishStatus, Metadata};

pub mod prelude {
	pub use crate::core::{
		application::Application,
		context::{RunOptions, Supervisor},
		environment::Environment,
		testing::{MockupOptions, Tester},
	};

	pub use crate::types::{
		address_book::AddressBook,
		machine::{Deposit, FinishStatus, Metadata, Output, PortalHandlerConfig},
		testing::{AdvanceResult, InspectResult, ResultUtils},
	};

	pub use crate::utils::{abi::abi, macros::*, units};
}
