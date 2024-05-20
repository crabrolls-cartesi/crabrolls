use super::application::Application;
use super::environment::Rollup;
use super::types::{AdvanceInputType, FinishStatus};
use std::error::Error;

use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct Context {
    cancellation_token: Arc<CancellationToken>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            cancellation_token: Arc::new(CancellationToken::new()),
        }
    }

    pub fn with_timeout(duration: Duration) -> Self {
        let ctx = Self::new();
        let cancellation_token = ctx.cancellation_token.clone();

        tokio::spawn(async move {
            sleep(duration).await;
            cancellation_token.cancel();
        });

        ctx
    }

    pub async fn done(&self) {
        self.cancellation_token.cancelled().await;
    }
}

pub struct RunOpts {
    pub rollup_url: String,
}

impl Default for RunOpts {
    fn default() -> Self {
        Self {
            rollup_url: "http://127.0.0.1:5004".to_string(),
        }
    }
}

pub async fn run(app: impl Application) -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let opts = RunOpts::default();
    let ctx = Context::new();
    let rollup = Rollup::new(opts.rollup_url.clone());

    let status = FinishStatus::Accept;

    info!(
        "Starting the application... Listening for inputs on {}",
        opts.rollup_url
    );

    loop {
        let input = rollup.finish_and_get_next(&ctx, status.clone()).await?;

        match input {
            Some(AdvanceInputType::Advance(advance_input)) => {
                debug!("Advance input: {:?}", advance_input);
                if let Err(e) = app
                    .advance(&rollup, advance_input.metadata, advance_input.payload)
                    .await
                {
                    error!("Error in advance: {}", e);
                    return Err(e);
                }
            }
            Some(AdvanceInputType::Inspect(inspect_input)) => {
                debug!("Inspect input: {:?}", inspect_input);
                if let Err(e) = app.inspect(&rollup, inspect_input.payload).await {
                    error!("Error in inspect: {}", e);
                    return Err(e);
                }
            }
            None => {
                debug!("Waiting for next input");
            }
        }
    }
}
