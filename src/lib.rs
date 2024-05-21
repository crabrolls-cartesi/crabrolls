extern crate pretty_env_logger;
#[macro_use]
extern crate log;

mod core;
mod utils;

pub use core::{application::Application, context::run, environment::Environment, types::Metadata};