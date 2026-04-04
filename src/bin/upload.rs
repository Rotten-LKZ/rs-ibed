use std::path::PathBuf;
use std::process;

use clap::Parser;

/// CLI tool to upload images to the rs-ibed server.
///
/// Examples:
///   upload -u http://localhost:3000 -t my_token photo.jpg
///   upload -u http://localhost:3000 -t my_token -k camera,time *.png
#[derive(Parser)]
#[command(name = "upload", version, about = "Upload images to rs-ibed server")]
struct Cli {
    /// API base URL (e.g. http://localhost:3000)
    #[arg(short = 'u', long, env = "UPLOAD_API_URL")]
    url: String,

    /// Authentication token (Bearer)
    #[arg(short = 't', long, env = "UPLOAD_AUTH_TOKEN")]
    token: String,

    /// Comma-separated metadata fields to keep.
    /// Available: camera, settings, time, copyright, location, others
    #[arg(short = 'k', long, value_name = "FIELDS")]
    keep_metadata_fields: Option<String>,

    /// Image file(s) to upload
    #[arg(required = true, value_name = "FILE")]
    files: Vec<PathBuf>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let api_url = format!("{}/api/upload", cli.url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let mut has_error = false;

    for path in &cli.files {
        if !path.exists() {
            eprintln!("[error] file not found: {}", path.display());
            has_error = true;
            continue;
        }

        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let data = match std::fs::read(path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[error] cannot read {}: {e}", path.display());
                has_error = true;
                continue;
            }
        };

        let mime = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();

        let file_part = reqwest::multipart::Part::bytes(data)
            .file_name(file_name.clone())
            .mime_str(&mime)
            .expect("invalid mime");

        let mut form = reqwest::multipart::Form::new().part("file", file_part);

        if let Some(ref fields) = cli.keep_metadata_fields {
            form = form.text("keep_metadata_fields", fields.clone());
        }

        match client
            .post(&api_url)
            .bearer_auth(&cli.token)
            .multipart(form)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();

                if status.is_success() {
                    let json: serde_json::Value =
                        serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);
                    let url = json["url"].as_str().unwrap_or("");
                    // If API returns full URL (starts with http), use it directly;
                    // otherwise, prepend the CLI-provided base URL
                    let full_url = if url.starts_with("http://") || url.starts_with("https://") {
                        url.to_string()
                    } else {
                        format!("{}{}", cli.url.trim_end_matches('/'), url)
                    };
                    println!("{full_url}");
                } else {
                    eprintln!("error: {} HTTP {status}: {body}", file_name);
                    has_error = true;
                }
            }
            Err(e) => {
                eprintln!("[error] {} -> {e}", file_name);
                has_error = true;
            }
        }
    }

    if has_error {
        process::exit(1);
    }
}
