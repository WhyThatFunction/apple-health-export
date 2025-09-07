use std::time::Duration;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "ahe-healthcheck")]
#[command(about = "Simple healthcheck client for apple-health-export", version)]
struct Args {
    /// Health URL to query
    #[arg(
        long,
        env = "AHE_HEALTH_URL",
        default_value = "http://127.0.0.1:8080/health"
    )]
    url: String,

    /// Timeout in milliseconds
    #[arg(long, env = "AHE_HEALTH_TIMEOUT_MS", default_value_t = 1500)]
    timeout_ms: u64,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let client = reqwest::Client::builder()
        .user_agent("ahe-healthcheck/1")
        .timeout(Duration::from_millis(args.timeout_ms))
        .build();

    let code = match client {
        Ok(client) => match client.get(&args.url).send().await {
            Ok(resp) if resp.status().is_success() => 0,
            Ok(resp) => {
                eprintln!("healthcheck failed: status {}", resp.status());
                1
            }
            Err(err) => {
                eprintln!("healthcheck error: {err}");
                2
            }
        },
        Err(err) => {
            eprintln!("failed to build client: {err}");
            3
        }
    };

    std::process::exit(code);
}
