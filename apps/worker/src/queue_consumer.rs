use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, warn};
use uuid::Uuid;

use crate::storage::Storage;
use crate::stomp::StompClient;
use crate::world::{generate, GenerateOptions};

#[derive(Deserialize)]
struct GenerateMessage {
    #[serde(default)]
    job_id: String,
    #[serde(default)]
    seed: Option<u32>,
    #[serde(default)]
    size: Option<u32>,
    #[serde(default)]
    scale: Option<u32>,
}

#[derive(Serialize)]
struct DoneMessage<'a> {
    job_id: &'a str,
    id: String,
    seed: u32,
    size: u32,
    scale: u32,
    png_url: String,
    png_key: String,
    json_url: String,
    json_key: String,
    duration_ms: u128,
}

pub async fn run_one(storage: Storage) -> Result<()> {
    let host = std::env::var("ARTEMIS_HOST").context("ARTEMIS_HOST not set")?;
    let port: u16 = std::env::var("ARTEMIS_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(61613);
    let user = std::env::var("ARTEMIS_USER").unwrap_or_else(|_| "artemis".into());
    let pass = std::env::var("ARTEMIS_PASSWORD").unwrap_or_else(|_| "artemis".into());
    let queue_in = std::env::var("QUEUE_IN").unwrap_or_else(|_| "world-gen".into());
    let queue_out = std::env::var("QUEUE_OUT").unwrap_or_else(|_| "world-done".into());
    let wait_secs: u64 = std::env::var("CONSUMER_WAIT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);

    let mut client = StompClient::connect(&host, port, &user, &pass).await?;
    info!(host, port, queue_in = %queue_in, "stomp connected");
    client
        .subscribe("worker-1", &queue_in, "client-individual")
        .await?;

    let frame = client.read_message(Duration::from_secs(wait_secs)).await?;
    let Some(frame) = frame else {
        warn!("no message within {wait_secs}s — exit clean");
        let _ = client.disconnect().await;
        return Ok(());
    };
    let ack_id = frame
        .header("ack")
        .map(|s| s.to_string())
        .or_else(|| frame.header("message-id").map(|s| s.to_string()))
        .unwrap_or_default();
    let body = frame.body_str().to_string();
    info!(bytes = body.len(), "message received");

    let msg: GenerateMessage = serde_json::from_str(&body).unwrap_or(GenerateMessage {
        job_id: String::new(),
        seed: None,
        size: None,
        scale: None,
    });

    let opts = GenerateOptions {
        seed: msg.seed.unwrap_or_else(rand::random),
        size: msg.size.unwrap_or(128).clamp(32, 512),
        scale: msg.scale.unwrap_or(8).clamp(1, 16),
    };

    let started = std::time::Instant::now();
    let result = tokio::task::spawn_blocking(move || generate(opts)).await??;

    let id = Uuid::new_v4().to_string();
    let png_key = format!("{}/{}.png", &id[..2], id);
    let json_key = format!("{}/{}.json", &id[..2], id);
    let png = storage.put(&png_key, result.png, "image/png").await?;
    let json_bytes = serde_json::to_vec(&result.json)?;
    let json = storage.put(&json_key, json_bytes, "application/json").await?;

    let done = DoneMessage {
        job_id: &msg.job_id,
        id: id.clone(),
        seed: opts.seed,
        size: opts.size,
        scale: opts.scale,
        png_url: png.url,
        png_key: png.key,
        json_url: json.url,
        json_key: json.key,
        duration_ms: started.elapsed().as_millis(),
    };
    let payload = serde_json::to_vec(&done)?;
    client
        .send(&queue_out, "application/json", &payload)
        .await?;

    if !ack_id.is_empty() {
        client.ack(&ack_id).await?;
    }
    let _ = client.disconnect().await;
    info!(id = %id, duration_ms = done.duration_ms, "job done");
    Ok(())
}
