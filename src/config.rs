use clap::Parser;
use std::string::ToString;

#[derive(Parser, Debug, Clone)]
#[command(name = "apple-health-export")]
#[command(about = "Axum service to ingest JSON and merge to S3 by day", version)]
pub struct Config {
    /// S3 bucket to store JSON files
    #[arg(long, env = "AHE_BUCKET", default_value_t = default_bucket_name())]
    pub bucket: String,

    /// Optional prefix inside the bucket (e.g. "exports/")
    #[arg(long, env = "AHE_PREFIX")]
    pub prefix: Option<String>,

    /// Bind address, e.g. 0.0.0.0:8080 or just port with --port
    #[arg(long, env = "AHE_BIND")]
    pub bind: Option<String>,

    /// Port to listen on (used if --bind not provided)
    #[arg(long, env = "AHE_PORT", default_value_t = 8080)]
    pub port: u16,

    /// Basic auth username
    #[arg(long, env = "AHE_BASIC_USER")]
    pub basic_user: Option<String>,

    /// Basic auth password
    #[arg(long, env = "AHE_BASIC_PASS")]
    pub basic_pass: Option<String>,

    /// Queue capacity for background ingestion
    #[arg(long, env = "AHE_QUEUE_CAP", default_value_t = 1024)]
    pub queue_cap: usize,

    /// Number of background worker tasks
    #[arg(long, env = "AHE_WORKERS", default_value_t = 1)]
    pub workers: usize,

    /// Use S3 path-style addressing (useful for MinIO/localstack)
    #[arg(long, env = "AHE_S3_PATH_STYLE", default_value_t = true)]
    pub s3_path_style: bool,
}

pub fn normalize_prefix(mut p: String) -> String {
    if !p.is_empty() && !p.ends_with('/') {
        p.push('/');
    }
    p
}

pub fn default_bucket_name() -> String {
    "user-apple-health-exports".to_string()
}
