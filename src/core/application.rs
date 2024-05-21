use super::environment::Environment;
use super::types::Metadata;
use std::error::Error;

pub trait Application {
    fn advance(
        &self,
        env: &impl Environment,
        metadata: Metadata,
        payload: Vec<u8>,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn Error>>>;

    fn inspect(
        &self,
        env: &impl Environment,
        payload: Vec<u8>,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn Error>>>;
}