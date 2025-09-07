use std::net::SocketAddr;

use aws_config::BehaviorVersion;
use aws_sdk_s3::Client as S3Client;
use axum::{
    Router, middleware,
    routing::{get, post},
};
use clap::Parser;
use mimalloc::MiMalloc;
use tracing::{debug, error, info};

mod auth;
mod config;
mod error;
mod handlers;
mod metrics;
mod s3;
mod state;
mod telemetry;

use crate::config::Config;
use crate::error::Result;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[allow(clippy::result_large_err)]
#[tokio::main]
async fn main() -> Result<()> {
    telemetry::init("apple-health-export").await;

    let cfg = Config::parse();
    debug!(
        bucket = %cfg.bucket,
        prefix = ?cfg.prefix,
        bind = ?cfg.bind,
        port = %cfg.port,
        workers = %cfg.workers,
        queue_cap = %cfg.queue_cap,
        s3_path_style = %cfg.s3_path_style,
        basic_auth_enabled = %cfg.basic_user.is_some() && cfg.basic_pass.is_some(),
        "Parsed configuration"
    );

    // AWS config via default chain (env, profile, etc.)
    let aws_config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let s3_conf = aws_sdk_s3::config::Builder::from(&aws_config)
        .force_path_style(cfg.s3_path_style)
        .build();
    let s3 = S3Client::from_conf(s3_conf);

    // Optionally validate access to bucket on startup (non-fatal)
    debug!(bucket = %cfg.bucket, "Validating access to S3 bucket");
    if let Err(e) = s3.head_bucket().bucket(&cfg.bucket).send().await {
        error!(error = ?e, bucket = %cfg.bucket, "Failed to access S3 bucket");
    } else {
        debug!(bucket = %cfg.bucket, "S3 bucket reachable");
    }

    let (app_state, rx) = state::build_state(&cfg, s3);
    let _workers = state::spawn_workers(app_state.clone(), rx, cfg.workers);
    debug!(workers = %cfg.workers, "Spawned worker tasks");

    // Build routers
    let ingest_router = Router::new()
        .route("/ingest", post(handlers::ingest))
        .route_layer(middleware::from_fn_with_state(
            app_state.clone(),
            auth::basic_auth,
        ));

    let app = Router::new()
        .route("/health", get(handlers::health))
        .merge(ingest_router)
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(app_state);

    let addr: SocketAddr = match cfg.bind {
        Some(bind_str) => bind_str.parse()?,
        None => ([0, 0, 0, 0], cfg.port).into(),
    };

    info!(%addr, "Starting server");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
