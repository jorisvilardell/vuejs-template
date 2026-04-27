mod queue_consumer;
mod storage;
mod stomp;
mod world;

use std::sync::Arc;
use tracing::info;

use storage::Storage;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,aws_sdk_s3=warn,aws_smithy=warn".into()),
        )
        .init();

    let storage = Arc::new(Storage::from_env().await?);
    info!(bucket = %storage.bucket, "storage ready");

    queue_consumer::run_one((*storage).clone()).await
}
