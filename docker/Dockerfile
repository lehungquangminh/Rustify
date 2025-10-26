# syntax=docker/dockerfile:1.7-labs
FROM --platform=$BUILDPLATFORM rust:1.81-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock* ./
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release || true
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release

FROM gcr.io/distroless/cc-debian12:nonroot
WORKDIR /app
USER nonroot
COPY --from=builder /app/target/release/rustify /app/rustify
COPY --from=builder /app/static /app/static
EXPOSE 8080
ENV RUST_LOG=info
ENTRYPOINT ["/app/rustify"]
