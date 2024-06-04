extern crate pretty_env_logger;
#[macro_use]
extern crate log;

mod core;
mod types;
mod utils;

pub use core::{
    application::Application,
    context::{run, RunOptions},
    environment::Environment,
    testing::Tester,
};

pub use types::{
    address::Address,
    machine::{FinishStatus, Metadata, Output},
};
