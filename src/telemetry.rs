use opentelemetry::{global, trace::TracerProvider as _};
use opentelemetry_otlp::{Protocol, WithExportConfig};
use opentelemetry_sdk::{Resource, metrics::SdkMeterProvider, trace::Sampler};
use tracing::debug;
use tracing_subscriber::{EnvFilter, Registry, prelude::*};

// Initialize tracing + OTLP export for traces & metrics.
pub async fn init(service_name: &str) {
    // Build a resource with service.name. Builder includes default detectors; if you
    // want to avoid env-detected values, use `builder_empty()` instead.
    let resource = Resource::builder()
        .with_service_name(service_name.to_string())
        .build();

    // Tracing exporter over gRPC (tonic). If building fails, continue without OTLP.
    let tracer_provider_opt = match opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
    {
        Ok(exporter) => {
            let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
                .with_batch_exporter(exporter)
                .with_resource(resource.clone())
                .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
                    1.0,
                ))))
                .build();
            global::set_tracer_provider(provider.clone());
            Some(provider)
        }
        Err(_e) => None,
    };

    // Metrics exporter over gRPC (tonic). Periodic export. Non-fatal on failure.
    if let Ok(metric_exporter) = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_protocol(Protocol::Grpc)
        .build()
    {
        let meter_provider = SdkMeterProvider::builder()
            .with_periodic_exporter(metric_exporter)
            .with_resource(resource)
            .build();
        global::set_meter_provider(meter_provider);
    }

    install_subscriber(service_name, tracer_provider_opt);
}

fn install_subscriber(
    service_name: &str,
    tracer_provider: Option<opentelemetry_sdk::trace::SdkTracerProvider>,
) {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("info,aws_config=warn,aws_smithy_runtime=warn,hyper=warn,tower_http=info")
    });

    let fmt_layer = tracing_subscriber::fmt::layer().with_target(false);

    // base subscriber
    let base = Registry::default().with(env_filter).with(fmt_layer);

    // Optionally add the OTEL layer
    if let Some(provider) = tracer_provider {
        let tracer = provider.tracer(service_name.to_string());
        tracing::subscriber::set_global_default(
            base.with(tracing_opentelemetry::layer().with_tracer(tracer)),
        )
        .expect("failed to install tracing subscriber");
        debug!("tracing subscriber with OTEL layer installed");
    } else {
        tracing::subscriber::set_global_default(base)
            .expect("failed to install tracing subscriber");
        debug!("tracing subscriber installed (no OTEL layer)");
    }
}
