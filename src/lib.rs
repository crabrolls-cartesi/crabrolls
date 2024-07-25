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
        context::{run, RunOptions},
        environment::Environment,
        testing::Tester,
    };

    pub use crate::types::{
        address::Address,
        machine::{Deposit, FinishStatus, Metadata, Output},
    };
}
