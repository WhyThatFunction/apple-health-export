# apple-health-export

Axum-based service that ingests JSON payloads and stores them in S3, merging by device and day. Each request appends JSON items to the object for the current UTC date under a per-device prefix. Optional basic auth, background workers, and OpenTelemetry tracing/metrics are included.

## Features

- JSON ingest endpoint that queues work and merges into S3.
- Daily S3 files per device: `prefix/<device>/<YYYY-MM-DD>.json`.
- Optional HTTP Basic Auth via environment variables.
- Background queue with configurable capacity and workers.
- OpenTelemetry traces and metrics (OTLP), plus structured logging.

## Quickstart

### Run with Docker Compose (local stack)

- Brings up the app, MinIO (S3-compatible), Jaeger, and the Otel Collector.
- Port mappings:
  - App: `http://localhost:8080`
  - MinIO: `http://localhost:9000` (console at `:9001`)
  - Jaeger UI: `http://localhost:16686`

Commands:

```
docker compose up --build
```

Note: `compose.yaml` mounts `otelcol.yaml` from repo root. A sample is in `.docker/otelcol.yaml`. Copy it to the root or update the compose file accordingly if needed.

### Run locally with Cargo

Prereqs: Rust toolchain and AWS credentials/config (or use an S3-compatible endpoint like MinIO).

```
# Example (uses env for AWS + config)
cargo run --bin ahe -- \
  --port 8080
```

Common AWS env vars:

- `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION`
- `AWS_ENDPOINT_URL` and `AWS_ALLOW_HTTP=true` (for MinIO/localstack)

### Healthcheck CLI

A small helper binary, `ahe-healthcheck`, pings the health endpoint (used in Dockerfile/compose):

```
cargo run --bin ahe-healthcheck -- --url http://127.0.0.1:8080/health
```

Env:

- `AHE_HEALTH_URL` (default `http://127.0.0.1:8080/health`)
- `AHE_HEALTH_TIMEOUT_MS` (default `1500`)

## API

### POST /ingest

Enqueue a payload for the given device. Items are merged into a JSON array in S3 for the current UTC day.

Request body:

```json
{
  "device_name": "apple-watch",
  "data": [
    { "type": "HeartRate", "bpm": 72, "ts": "2025-09-07T13:37:00Z" },
    { "type": "Steps", "count": 560, "ts": "2025-09-07T13:45:00Z" }
  ]
}
```

Responses:

- `202 Accepted` when queued
- `503 Service Unavailable` when the queue is full
- `401 Unauthorized` if basic auth is required and missing/invalid

Auth:

- If `AHE_BASIC_USER` and `AHE_BASIC_PASS` are set, include a Basic header:

```
Authorization: Basic base64(username:password)
```

Example:

```
curl -i -X POST http://localhost:8080/ingest \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Basic <a-basic-auth>' \
  -d '{
        "device_name": "apple-watch",
        "data": [ {"foo": 1}, {"bar": 2} ]
      }'
```

### GET /health

- Returns `200 OK` with body `ok`.

## S3 Object Layout

- Key format: `prefix/<device>/<YYYY-MM-DD>.json` (prefix optional)
- The day uses the server’s current UTC date.
- `device_name` is sanitized to a safe path segment.
- Merge semantics:
  - If an existing object is an array and new data is an array, items are appended.
  - Mixed non-array/array inputs are coerced to an array with all items preserved.

## Configuration

All settings are available via CLI flags and/or environment variables (shown below with env names and defaults where applicable):

- `--bucket` / `AHE_BUCKET`: S3 bucket (default: `user-apple-health-exports`).
- `--prefix` / `AHE_PREFIX`: Optional key prefix inside the bucket (e.g. `exports/`).
- `--bind` / `AHE_BIND`: Bind address (e.g. `0.0.0.0:8080`).
- `--port` / `AHE_PORT`: Port if `--bind` is not given (default: `8080`).
- `--basic-user` / `AHE_BASIC_USER`: Basic auth username (optional).
- `--basic-pass` / `AHE_BASIC_PASS`: Basic auth password (optional).
- `--queue-cap` / `AHE_QUEUE_CAP`: Queue capacity for background ingestion (default: `1024`).
- `--workers` / `AHE_WORKERS`: Number of background worker tasks (default: `1`).
- `--s3-path-style` / `AHE_S3_PATH_STYLE`: Use path-style addressing (default: `true`, useful for MinIO/localstack).

OpenTelemetry (OTLP) examples (all optional):

- `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT`, `OTEL_EXPORTER_OTLP_TRACES_PROTOCOL`
- `OTEL_EXPORTER_OTLP_METRICS_ENDPOINT`, `OTEL_EXPORTER_OTLP_METRICS_PROTOCOL`
- `RUST_LOG` for log verbosity (e.g. `info,aws_config=warn,hyper=warn,tower_http=info`)

## Build Container Image

The provided `Dockerfile` builds statically linked binaries and produces a minimal distroless image:

```
docker build -t ahe:prod .
```

The final image exposes `8080` and runs as non-root with the `apple-health-export` entrypoint.

## Development

- Pre-commit hooks run `cargo check`, `cargo fmt`, and `cargo clippy`.
- Profiles include an optimized `prod` profile (LTO, strip, minimal size).

Useful commands:

```
cargo check --all-targets --all-features
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo run --bin ahe -- --port 8080
```

## License

MIT
