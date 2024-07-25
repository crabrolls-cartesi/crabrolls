use crabrolls::prelude::*;
use std::error::Error;

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
        payload: &[u8],
    ) -> Result<FinishStatus, Box<dyn Error>> {
        println!(
            "Advance method called with payload: {:?}",
            String::from_utf8_lossy(&payload)
        );
        env.send_notice(payload.clone()).await?;
        env.send_report(payload.clone()).await?;
        env.send_voucher(metadata.sender, payload).await?;
        Ok(FinishStatus::Accept)
    }

    async fn inspect(
        &self,
        env: &impl Environment,
        payload: &[u8],
    ) -> Result<FinishStatus, Box<dyn Error>> {
        println!(
            "Inspect method called with payload: {:?}",
            String::from_utf8_lossy(&payload)
        );
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

#[cfg(test)]
mod tests {
    use super::EchoApp;
    use crabrolls::prelude::*;

    #[async_std::test]
    async fn test_echo() {
        let app = EchoApp::new();
        let tester = Tester::new(app);

        let address = Address::default();

        let payload = b"Hi Crabrolls!".to_vec();
        let result = tester.advance(address, payload.clone()).await;

        assert_eq!(
            result.status,
            FinishStatus::Accept,
            "Expected Accept status"
        );

        assert!(result.error.is_none(), "Expected no error");

        assert_eq!(
            result.outputs.len(),
            3,
            "Expected 3 outputs, got {}",
            result.outputs.len()
        );

        assert_eq!(
            result.outputs,
            vec![
                Output::Notice {
                    payload: payload.clone()
                },
                Output::Report {
                    payload: payload.clone()
                },
                Output::Voucher {
                    destination: address,
                    payload: payload.clone()
                }
            ],
            "Expected outputs to match"
        );

        assert_eq!(
            result.metadata.sender,
            Address::default(),
            "Unexpected sender address"
        );
    }
}
