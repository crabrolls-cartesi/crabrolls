use super::environment::Environment;
use crate::types::machine::{FinishStatus, Metadata};
use std::{error::Error, future::Future};

pub trait Application {
    fn advance(
        &self,
        env: &impl Environment,
        metadata: Metadata,
        payload: &[u8],
    ) -> impl Future<Output = Result<FinishStatus, Box<dyn Error>>>;

    fn inspect(
        &self,
        env: &impl Environment,
        payload: &[u8],
    ) -> impl Future<Output = Result<FinishStatus, Box<dyn Error>>>;
}
