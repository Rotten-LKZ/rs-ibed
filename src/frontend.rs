use axum::extract::Request;
use axum::response::Response;

// ---------------------------------------------------------------------------
// Production: serve frontend files embedded in the binary
// ---------------------------------------------------------------------------
#[cfg(not(debug_assertions))]
mod embedded {
    use axum::body::Body;
    use axum::http::{StatusCode, header};
    use axum::response::{IntoResponse, Response};
    use rust_embed::Embed;

    #[derive(Embed)]
    #[folder = "frontend/build"]
    struct Assets;

    pub async fn serve(uri: &axum::http::Uri) -> Response {
        let path = uri.path().trim_start_matches('/');
        let path = if path.is_empty() { "index.html" } else { path };

        if let Some(file) = Assets::get(path) {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let body = Body::from(file.data.into_owned());
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(body)
                .unwrap()
        } else if let Some(fallback) = Assets::get("index.html") {
            let body = Body::from(fallback.data.into_owned());
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/html")
                .body(body)
                .unwrap()
        } else {
            StatusCode::NOT_FOUND.into_response()
        }
    }
}

// ---------------------------------------------------------------------------
// Development: reverse-proxy to the Vite dev server on port 6492
// ---------------------------------------------------------------------------
#[cfg(debug_assertions)]
mod proxy {
    use axum::body::Body;
    use axum::extract::Request;
    use axum::http::{StatusCode, header};
    use axum::response::{IntoResponse, Response};

    const DEV_SERVER: &str = "http://127.0.0.1:6492";

    static CLIENT: std::sync::LazyLock<reqwest::Client> = std::sync::LazyLock::new(|| {
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .build()
            .expect("failed to build reqwest client for dev proxy")
    });

    pub async fn forward(req: Request) -> Response {
        let path_and_query = req
            .uri()
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/");
        let url = format!("{DEV_SERVER}{path_and_query}");

        let method = req.method().clone();
        let headers = req.headers().clone();
        let body = axum::body::to_bytes(req.into_body(), 10 * 1024 * 1024)
            .await
            .unwrap_or_default();

        let mut builder = CLIENT.request(method, &url);
        for (name, value) in headers.iter() {
            if name != header::HOST {
                builder = builder.header(name.clone(), value.clone());
            }
        }
        if !body.is_empty() {
            builder = builder.body(body);
        }

        match builder.send().await {
            Ok(resp) => {
                let status = StatusCode::from_u16(resp.status().as_u16())
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                let resp_headers = resp.headers().clone();
                let bytes = resp.bytes().await.unwrap_or_default();

                let mut response = Response::builder().status(status);
                for (name, value) in resp_headers.iter() {
                    response = response.header(name, value);
                }
                response.body(Body::from(bytes)).unwrap_or_else(|_| {
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                })
            }
            Err(e) => {
                tracing::warn!("dev proxy error: {e}");
                (
                    StatusCode::BAD_GATEWAY,
                    "Frontend dev server (port 6492) is not reachable. Run: cd frontend && pnpm dev",
                )
                    .into_response()
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Public handler dispatched from the router fallback
// ---------------------------------------------------------------------------
pub async fn handler(req: Request) -> Response {
    #[cfg(debug_assertions)]
    {
        proxy::forward(req).await
    }
    #[cfg(not(debug_assertions))]
    {
        let uri = req.uri().clone();
        embedded::serve(&uri).await
    }
}
