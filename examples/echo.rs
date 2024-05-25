use std::error::Error;
extern crate crabrolls;
use async_std::prelude::*;
use crabrolls::{run, Application, Environment, FinishStatus, Metadata, RunOptions};

struct EchoApp;

impl EchoApp {
    fn new() -> Self {
        Self
    }
}

impl Application for EchoApp {
    async fn advance(
        &self,
        env: &impl Environment,
        metadata: Metadata,
        payload: Vec<u8>,
    ) -> Result<FinishStatus, Box<dyn Error>> {
        let string_payload = String::from_utf8_lossy(&payload);
        println!("Advance method called with payload: {:?}", string_payload);
        env.send_notice(payload.clone()).await?;
        env.send_report(payload.clone()).await?;
        env.send_voucher(metadata.msg_sender, payload).await?;
        Ok(FinishStatus::Accept)
    }

    async fn inspect(
        &self,
        env: &impl Environment,
        payload: Vec<u8>,
    ) -> Result<FinishStatus, Box<dyn Error>> {
        env.send_report(payload).await?;
        Ok(FinishStatus::Accept)
    }
}

#[async_std::main]
async fn main() {
    let app = EchoApp::new();
    let options = RunOptions::default();
    if let Err(e) = run(app, options).await {
        eprintln!("Error: {}", e);
    }
}
