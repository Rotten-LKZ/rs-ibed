FROM node:24-bookworm-slim AS frontend-builder
WORKDIR /work/frontend
COPY frontend/package.json frontend/pnpm-lock.yaml ./
RUN corepack enable && pnpm install --frozen-lockfile
COPY frontend/ ./
RUN pnpm build

FROM rust:1.94-bookworm AS rust-builder
WORKDIR /work
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY config.toml ./config.toml
COPY openapi.json ./openapi.json
COPY --from=frontend-builder /work/frontend/build ./frontend/build
RUN cargo build --release --bin rs-ibed

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=rust-builder /work/target/release/rs-ibed /usr/local/bin/rs-ibed
EXPOSE 3000
ENTRYPOINT ["rs-ibed"]
CMD ["--config", "/app/config.toml"]
