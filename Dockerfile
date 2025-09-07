ARG RUST_VERSION=1

# Build statically-linked binaries using musl on Alpine per-arch
FROM --platform=$BUILDPLATFORM rust:${RUST_VERSION}-alpine AS builder

RUN apk add --no-cache musl-dev build-base pkgconfig

WORKDIR /app

# Build with BuildKit cache mounts for speed
ARG TARGETPLATFORM
RUN case "$TARGETPLATFORM" in \
      "linux/amd64")  rustup target add x86_64-unknown-linux-musl && echo x86_64-unknown-linux-musl > /rust_target ;; \
      "linux/arm64")  rustup target add aarch64-unknown-linux-musl && echo aarch64-unknown-linux-musl > /rust_target ;; \
      *) echo "unsupported target $TARGETPLATFORM"; exit 1 ;; \
    esac

ENV RUSTFLAGS="-C target-feature=+crt-static"
# Build using bind-mounted source (ro) and cached target dir
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    --mount=type=bind,source=.,target=/src,ro \
    CARGO_TARGET_DIR=/app/target cargo build --profile prod --locked \
      --bin ahe --bin ahe-healthcheck \
      --target $(cat /rust_target) \
      --manifest-path /src/Cargo.toml \
 && mkdir -p /out \
 && cp target/$(cat /rust_target)/prod/ahe /out/apple-health-export \
 && cp target/$(cat /rust_target)/prod/ahe-healthcheck /out/ahe-healthcheck
# Minimal CA bundle for TLS where needed
FROM alpine:3.20 AS certs
RUN apk add --no-cache ca-certificates

# Final static image using Distroless (nonroot)
FROM gcr.io/distroless/static-debian12:nonroot AS runtime

ENV RUST_LOG=info
EXPOSE 8080

COPY --from=builder /out/apple-health-export /usr/local/bin/apple-health-export
COPY --from=builder /out/ahe-healthcheck /usr/local/bin/ahe-healthcheck
COPY --from=certs /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt

# Ensure we run as the distroless nonroot user
USER nonroot:nonroot

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD ["/usr/local/bin/ahe-healthcheck"]

ENTRYPOINT ["/usr/local/bin/apple-health-export"]
CMD ["--port", "8080"]
