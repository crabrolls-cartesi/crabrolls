use std::error::Error;
use tokio;
extern crate crabrolls;
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let options = RunOptions::default();

    run(EchoApp::new(), options).await
}
