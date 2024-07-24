use async_std::sync::{Mutex, RwLock};
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
    outputs: RwLock<Vec<Output>>,
    input_index: Mutex<i32>,
}

impl RollupMockup {
    pub fn new() -> Self {
        RollupMockup {
            outputs: RwLock::new(Vec::new()),
            input_index: Mutex::new(0),
        }
    }

    pub async fn handle(&self, output: Output) -> Result<i32, Box<dyn Error>> {
        let mut outputs = self.outputs.write().await;
        outputs.push(output);
        Ok(outputs.len().try_into()?)
    }

    async fn reset(&self) {
        let mut outputs = self.outputs.write().await;
        outputs.clear();
    }

    pub async fn advance(
        &self,
        status: FinishStatus,
    ) -> Result<Option<Vec<Output>>, Box<dyn Error>> {
        let mut input_index = self.input_index.lock().await;
        *input_index += 1;

        let outputs = self.outputs.read().await.clone();
        self.reset().await;

        match status {
            FinishStatus::Accept => Ok(Some(outputs)),
            FinishStatus::Reject => Ok(None),
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
        self.handle(Output::Voucher {
            destination,
            payload,
        })
        .await
    }

    async fn send_notice(&self, payload: Vec<u8>) -> Result<i32, Box<dyn Error>> {
        self.handle(Output::Notice { payload }).await
    }

    async fn send_report(&self, payload: Vec<u8>) -> Result<(), Box<dyn Error>> {
        self.handle(Output::Report { payload }).await?;
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
            sender: sender,
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
            outputs: match self.env.advance(status).await {
                Ok(Some(outputs)) => outputs,
                _ => Vec::new(),
            },
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
            outputs: self.env.outputs.read().await.clone(),
            error,
        }
    }
}
