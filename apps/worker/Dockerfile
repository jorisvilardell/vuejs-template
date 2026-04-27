# syntax=docker/dockerfile:1.7

FROM rust:1.94-slim-bookworm AS build
WORKDIR /src
RUN apt-get update && apt-get install -y --no-install-recommends \
      pkg-config libssl-dev ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# pre-cache deps
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo "fn main(){}" > src/main.rs \
 && cargo build --release --locked \
 && rm -rf src target/release/deps/worker* target/release/worker target/release/worker.d

# real build
COPY src ./src
RUN find src -exec touch {} + \
 && cargo build --release --locked \
 && strip target/release/worker

FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
WORKDIR /app
COPY --from=build /src/target/release/worker /app/worker
USER nonroot:nonroot
ENV PORT=3001 \
    RUST_LOG=info,aws=warn \
    MODE=http
EXPOSE 3001
ENTRYPOINT ["/app/worker"]
