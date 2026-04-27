mod queue_consumer;
mod storage;
mod stomp;
mod world;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tower_http::cors::CorsLayer;
use tracing::{error, info, instrument};
use uuid::Uuid;

use storage::Storage;
use world::{generate, GenerateOptions};

#[derive(Clone)]
struct AppState {
    storage: Arc<Storage>,
}

#[derive(Deserialize, Default)]
struct GenerateQuery {
    seed: Option<u32>,
    size: Option<u32>,
    scale: Option<u32>,
}

#[derive(Serialize)]
struct WorldResponse {
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

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
    service: &'static str,
}

#[instrument(skip_all)]
async fn health() -> Json<Health> {
    Json(Health { status: "ok", service: "worker" })
}

#[instrument(skip_all, fields(seed, size, scale))]
async fn handle_generate(
    State(state): State<AppState>,
    Query(q): Query<GenerateQuery>,
) -> Result<Json<WorldResponse>, (StatusCode, Json<ErrorResponse>)> {
    let opts = GenerateOptions {
        seed: q.seed.unwrap_or_else(rand::random),
        size: q.size.unwrap_or(128).clamp(32, 512),
        scale: q.scale.unwrap_or(8).clamp(1, 16),
    };
    let started = std::time::Instant::now();
    let result = tokio::task::spawn_blocking(move || generate(opts))
        .await
        .map_err(internal)?
        .map_err(internal)?;

    let id = Uuid::new_v4().to_string();
    let png_key = format!("{}/{}.png", &id[..2], id);
    let json_key = format!("{}/{}.json", &id[..2], id);
    let json_bytes = serde_json::to_vec(&result.json).map_err(internal)?;

    let png_up = state
        .storage
        .put(&png_key, result.png, "image/png")
        .await
        .map_err(internal)?;
    let json_up = state
        .storage
        .put(&json_key, json_bytes, "application/json")
        .await
        .map_err(internal)?;

    Ok(Json(WorldResponse {
        id,
        seed: opts.seed,
        size: opts.size,
        scale: opts.scale,
        png_url: png_up.url,
        png_key: png_up.key,
        json_url: json_up.url,
        json_key: json_up.key,
        duration_ms: started.elapsed().as_millis(),
    }))
}

fn internal<E: std::fmt::Display>(e: E) -> (StatusCode, Json<ErrorResponse>) {
    error!(error = %e, "request failed");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse { error: e.to_string() }),
    )
}

async fn run_http(storage: Arc<Storage>) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/healthz", get(health))
        .route("/generate", post(handle_generate))
        .with_state(AppState { storage })
        .layer(CorsLayer::permissive());

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3001);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!(?addr, "worker http listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=info,aws_sdk_s3=warn,aws_smithy=warn".into()),
        )
        .init();

    let storage = Arc::new(Storage::from_env().await?);
    info!(bucket = %storage.bucket, "storage ready");

    let mode = std::env::var("MODE").unwrap_or_else(|_| "http".into()).to_lowercase();
    info!(mode, "worker mode");

    match mode.as_str() {
        "queue" => queue_consumer::run_one((*storage).clone()).await,
        _ => run_http(storage).await,
    }
}
