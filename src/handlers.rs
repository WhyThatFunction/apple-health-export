use axum::{
    Json,
    extract::State,
    http::{Method, StatusCode},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tracing::{debug, instrument};

use crate::metrics;
use crate::s3::IngestJob;
use crate::state::AppState;

#[instrument(skip_all)]
pub async fn health() -> impl IntoResponse {
    debug!("health check invoked");
    (StatusCode::OK, "ok")
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IngestRequest {
    pub device_name: String,
    pub data: Vec<JsonValue>,
}

#[instrument(
    skip(state, payload, method),
    fields(
        http_method = %method,
        device_name = %payload.device_name,
        items = %payload.data.len()
    )
)]
pub async fn ingest(
    method: Method,
    State(state): State<AppState>,
    Json(payload): Json<IngestRequest>,
) -> impl IntoResponse {
    // Metrics: count incoming requests to /ingest by method and device
    metrics::inc_ingest_request(method.as_str(), Some(&payload.device_name));
    debug!(device = %payload.device_name, items = payload.data.len(), "enqueueing ingest job");
    // Enqueue the job for background processing
    let job = IngestJob {
        device_name: payload.device_name,
        payload: JsonValue::Array(payload.data),
    };
    match state.tx.try_send(job) {
        Ok(()) => {
            debug!("job queued successfully");
            (StatusCode::ACCEPTED, "queued")
        }
        Err(err) => {
            use tokio::sync::mpsc::error::TrySendError;
            let code = match err {
                TrySendError::Full(_) => {
                    debug!("job queue is full");
                    StatusCode::SERVICE_UNAVAILABLE
                }
                TrySendError::Closed(_) => {
                    debug!("job queue channel is closed");
                    StatusCode::INTERNAL_SERVER_ERROR
                }
            };
            (code, "unavailable")
        }
    }
}
