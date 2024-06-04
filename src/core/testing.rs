use async_std::sync::Mutex;
use std::{error::Error, time::UNIX_EPOCH};

use crate::{
    types::{
        address::Address,
        machine::{FinishStatus, Output},
        testing::{AdvanceResult, InspectResult},
    },
    Application, Environment, Metadata,
};

pub struct RollupMockup {
    outputs: Mutex<Vec<Output>>,
    input_index: Mutex<i32>,
}

impl RollupMockup {
    pub fn new() -> Self {
        RollupMockup {
            outputs: Mutex::new(Vec::new()),
            input_index: Mutex::new(0),
        }
    }

    pub async fn handle(&self, output: Output) -> Option<i32> {
        self.outputs.lock().await.push(output);

        Some(
            self.outputs
                .lock()
                .await
                .len()
                .try_into()
                .expect("Failed to convert usize to i32"),
        )
    }

    async fn reset(&self) {
        self.outputs.lock().await.clear();
    }

    pub async fn advance(&self, status: FinishStatus) -> Option<Vec<Output>> {
        *self.input_index.lock().await += 1;

        let outputs = self.outputs.lock().await.clone();
        self.reset().await;

        match status {
            FinishStatus::Accept => Some(outputs),
            FinishStatus::Reject => None,
        }
    }

    pub async fn get_input_index(&self) -> i32 {
        *self.input_index.lock().await
    }
}

impl Environment for RollupMockup {
    async fn send_voucher(
        &self,
        destination: Address,
        payload: Vec<u8>,
    ) -> Result<i32, Box<dyn Error>> {
        let output = Output::Voucher {
            destination: destination,
            payload,
        };
        let index = self.handle(output).await.expect("Failed to send voucher");
        Ok(index)
    }

    async fn send_notice(&self, payload: Vec<u8>) -> Result<i32, Box<dyn Error>> {
        let output = Output::Notice { payload };
        let index = self.handle(output).await.expect("Failed to send notice");
        Ok(index)
    }

    async fn send_report(&self, payload: Vec<u8>) -> Result<(), Box<dyn Error>> {
        let output = Output::Report { payload };
        self.handle(output).await.expect("Failed to send report");
        Ok(())
    }
}

pub struct Tester<A> {
    app: A,
    env: RollupMockup,
}

impl<A> Tester<A>
where
    A: Application,
{
    pub fn new(app: A) -> Self {
        Self {
            app,
            env: RollupMockup::new(),
        }
    }

    pub async fn advance(&self, sender: Address, payload: Vec<u8>) -> AdvanceResult {
        let metadata = Metadata {
            input_index: self.env.get_input_index().await,
            msg_sender: sender,
            block_number: self.env.get_input_index().await as i64,
            timestamp: UNIX_EPOCH.elapsed().unwrap().as_secs() as i64,
        };

        let (status, error) = match self
            .app
            .advance(&self.env, metadata.clone(), payload.clone())
            .await
        {
            Ok(finish_status) => (finish_status, None),
            Err(e) => (FinishStatus::Reject, Some(e)),
        };

        AdvanceResult {
            status,
            outputs: self.env.advance(status).await.unwrap_or_default(),
            metadata,
            error,
        }
    }

    pub async fn inspect(&self, payload: Vec<u8>) -> InspectResult {
        let (status, error) = match self.app.inspect(&self.env, payload.clone()).await {
            Ok(finish_status) => (finish_status, None),
            Err(e) => (FinishStatus::Reject, Some(e)),
        };

        InspectResult {
            status,
            outputs: self.env.outputs.lock().await.clone(),
            error,
        }
    }
}
