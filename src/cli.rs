use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rs-ibed", about = "Image hosting service")]
pub struct Cli {
    /// Path to the configuration file
    #[arg(short, long, default_value = "config.toml")]
    pub config: String,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Export the OpenAPI spec to a JSON file without starting the server
    ExportOpenapi {
        /// Output path for the generated OpenAPI JSON
        #[arg(default_value = "openapi.json")]
        output: PathBuf,
    },
}
