FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY dist/rs-ibed /usr/local/bin/rs-ibed
EXPOSE 3000
ENTRYPOINT ["rs-ibed"]
CMD ["--config", "/app/config.toml"]
