use super::environment::Environment;
use crate::types::machine::{Deposit, FinishStatus, Metadata};
use std::{error::Error, future::Future};

pub trait Application {
    fn advance(
        &self,
        env: &impl Environment,
        metadata: Metadata,
        payload: Vec<u8>,
        deposit: Option<Deposit>,
    ) -> impl Future<Output = Result<FinishStatus, Box<dyn Error>>>;

    fn inspect(
        &self,
        env: &impl Environment,
        payload: Vec<u8>,
    ) -> impl Future<Output = Result<FinishStatus, Box<dyn Error>>>;
}
