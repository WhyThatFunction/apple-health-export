use aws_sdk_s3::operation::get_object::GetObjectError;
use aws_sdk_s3::{error::SdkError, primitives::ByteStream};
use chrono::{Datelike, NaiveDate, Utc};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

use crate::error::Result;
use crate::metrics;
use crate::state::AppState;

#[instrument(skip(prefix, device_name))]
pub fn s3_key_for_device_date(
    prefix: &Option<String>,
    device_name: &str,
    date: NaiveDate,
) -> String {
    let dev = sanitize_path_segment(device_name);
    let filename = format!(
        "{:04}-{:02}-{:02}.json",
        date.year(),
        date.month(),
        date.day()
    );
    match prefix {
        Some(p) => format!("{}{}/{}", p, dev, filename),
        None => format!("{}/{}", dev, filename),
    }
}

#[instrument(skip(state, new_json))]
pub async fn save_or_merge_json(state: &AppState, key: &str, new_json: JsonValue) -> Result<()> {
    // Try to fetch existing object
    debug!(%key, "checking existing object");
    let existing_json = match state
        .s3
        .get_object()
        .bucket(&state.bucket)
        .key(key)
        .send()
        .await
    {
        Ok(obj) => {
            let bytes = obj.body.collect().await?.into_bytes();
            let text = String::from_utf8(bytes.to_vec())?;
            debug!(%key, bytes = text.len(), "existing object found");
            Some(serde_json::from_str::<JsonValue>(&text)?)
        }
        Err(err) => {
            if is_s3_not_found(&err) {
                debug!(%key, "no existing object (will create new)");
                None
            } else {
                return Err(err.into());
            }
        }
    };

    let merged = match existing_json {
        None => new_json,
        Some(old) => merge_json(old, new_json),
    };

    let body = serde_json::to_vec_pretty(&merged)?;
    let items_after = match &merged {
        JsonValue::Array(a) => a.len(),
        _ => 1,
    };
    debug!(%key, items_after, bytes = body.len(), "writing merged JSON to S3");
    state
        .s3
        .put_object()
        .bucket(&state.bucket)
        .key(key)
        .content_type("application/json")
        .body(ByteStream::from(body))
        .send()
        .await?;
    debug!(%key, "put_object completed");

    Ok(())
}

#[instrument(skip(existing, incoming))]
pub fn merge_json(existing: JsonValue, incoming: JsonValue) -> JsonValue {
    match (existing, incoming) {
        (JsonValue::Array(mut a), JsonValue::Array(mut b)) => {
            a.append(&mut b);
            JsonValue::Array(a)
        }
        (JsonValue::Array(mut a), b) => {
            a.push(b);
            JsonValue::Array(a)
        }
        (a, JsonValue::Array(mut b)) => {
            let mut arr = vec![a];
            arr.append(&mut b);
            JsonValue::Array(arr)
        }
        (a, b) => JsonValue::Array(vec![a, b]),
    }
}

pub fn is_s3_not_found(err: &SdkError<GetObjectError>) -> bool {
    err.as_service_error()
        .map(|e| e.is_no_such_key())
        .unwrap_or(false)
}

// Background job processing
#[derive(Debug)]
pub struct IngestJob {
    pub device_name: String,
    pub payload: JsonValue,
}

#[instrument(skip(state, job), fields(device_name = %job.device_name))]
pub async fn process_job(state: Arc<AppState>, job: IngestJob) {
    let today = Utc::now().date_naive();
    let key = s3_key_for_device_date(&state.prefix, &job.device_name, today);
    // Track jobs in-flight via a gauge-like up/down counter
    metrics::inc_jobs_inflight();
    let res = save_or_merge_json(&state, &key, job.payload).await;
    metrics::dec_jobs_inflight();

    match res {
        Ok(()) => {
            info!(%key, device = %job.device_name, "stored payload");
        }
        Err(err) => {
            error!(error=?err, %key, device = %job.device_name, "failed to store payload");
        }
    }
}

fn sanitize_path_segment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '/' | '\\' | '?' | '#' | '%' | '"' | '<' | '>' | '|' | ':' => out.push('_'),
            _ => out.push(ch),
        }
    }
    out
}
