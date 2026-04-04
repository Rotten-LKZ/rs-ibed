mod auth;
mod cli;
mod config;
mod db;
mod error;
mod exif;
mod file_type;
mod frontend;
mod handlers;
mod image_proc;
mod models;
mod path_parser;
mod router;
mod server;
mod state;
mod storage;
mod storage_backend;

use clap::Parser;
use cli::{Cli, Command};
use config::AppConfig;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

fn init_tracing(log_level: &str) {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level)))
        .with(fmt::layer())
        .init();
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    auth::install_crypto_provider();

    let cli: Cli = Cli::parse();

    let (config, secrets) = AppConfig::load(&cli.config)
        .unwrap_or_else(|e| panic!("Failed to load {}: {e}", cli.config));

    init_tracing(&config.server.log_level.to_string());

    match cli.command {
        Some(Command::ExportOpenapi { output }) => {
            let spec = router::openapi_spec();
            let bytes = serde_json::to_vec_pretty(&spec)
                .unwrap_or_else(|e| panic!("Failed to serialize OpenAPI spec: {e}"));
            std::fs::write(&output, bytes)
                .unwrap_or_else(|e| panic!("Failed to write {}: {e}", output.display()));
            tracing::info!(path = %output.display(), "OpenAPI spec exported");
        }
        None => server::run(config, secrets).await,
    }
}
