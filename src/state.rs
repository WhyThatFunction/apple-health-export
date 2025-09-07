use aws_sdk_s3::Client as S3Client;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::debug;

use crate::config::Config;
use crate::config::normalize_prefix;
use crate::s3::IngestJob;

#[derive(Clone)]
pub struct AppState {
    pub s3: S3Client,
    pub bucket: String,
    pub prefix: Option<String>,
    pub basic_auth: Option<String>, // stored as "user:pass"
    pub tx: mpsc::Sender<IngestJob>,
}

pub struct WorkerHandles {
    #[allow(dead_code)]
    pub join_handles: Vec<tokio::task::JoinHandle<()>>,
}

pub fn build_state(config: &Config, s3: S3Client) -> (AppState, mpsc::Receiver<IngestJob>) {
    let (tx, rx) = mpsc::channel::<IngestJob>(config.queue_cap);
    let basic_auth = match (&config.basic_user, &config.basic_pass) {
        (Some(u), Some(p)) => Some(format!("{}:{}", u, p)),
        _ => None,
    };
    debug!(
        bucket = %config.bucket,
        prefix = ?config.prefix,
        queue_cap = %config.queue_cap,
        workers = %config.workers,
        basic_auth_enabled = %basic_auth.is_some(),
        "AppState constructed"
    );
    (
        AppState {
            s3,
            bucket: config.bucket.clone(),
            prefix: config.prefix.clone().map(normalize_prefix),
            basic_auth,
            tx,
        },
        rx,
    )
}

pub fn spawn_workers(
    state: AppState,
    rx: mpsc::Receiver<IngestJob>,
    workers: usize,
) -> WorkerHandles {
    let mut join_handles = Vec::with_capacity(workers.max(1));
    let rx = Arc::new(tokio::sync::Mutex::new(rx));
    for _ in 0..workers.max(1) {
        let state = Arc::new(state.clone());
        let rx_shared = rx.clone();
        let handle = tokio::spawn(async move {
            debug!("worker started");
            loop {
                let job_opt = {
                    let mut guard = rx_shared.lock().await;
                    guard.recv().await
                };
                match job_opt {
                    Some(job) => {
                        debug!(device = %job.device_name, "worker received job");
                        crate::s3::process_job(state.clone(), job).await
                    }
                    None => {
                        debug!("worker channel closed; exiting");
                        break;
                    }
                }
            }
        });
        join_handles.push(handle);
    }
    WorkerHandles { join_handles }
}
