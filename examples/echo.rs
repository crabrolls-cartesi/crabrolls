use std::error::Error;
use tokio;
extern crate crabrolls;
use crabrolls::{run, Application, Environment, Metadata};

struct EchoApp;

impl EchoApp {
    fn new() -> Self {
        Self
    }
}

impl Application for EchoApp {
    async fn advance(
        &self,
        _env: &impl Environment,
        _metadata: Metadata,
        _payload: Vec<u8>,
    ) -> Result<(), Box<dyn Error>> {
        println!("Advance method called with metadata: {:?}", _metadata);
        let string_payload = String::from_utf8_lossy(&_payload);
        println!("Advance method called with payload: {:?}", string_payload);
        Ok(())
    }

    async fn inspect(
        &self,
        _env: &impl Environment,
        _payload: Vec<u8>,
    ) -> Result<(), Box<dyn Error>> {
        println!("Inspect method called with payload: {:?}", _payload);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    run(EchoApp::new()).await
}
