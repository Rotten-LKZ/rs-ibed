use axum::http::HeaderValue;
use axum::{
    routing::{get, post},
    Json, Router,
};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::set_header::SetResponseHeaderLayer;
use utoipa::OpenApi;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::openapi::{ComponentsBuilder, OpenApi as UtoipaOpenApi};

use crate::handlers;
use crate::handlers::auth::{AuthCheckResponse, AuthSuccessResponse, CliLoginQuery, LoginRequest};
use crate::handlers::upload::UploadRequest;
use crate::models::image::{
    ImageCountResponse, ImageDetailResponse, ImageListQuery, ImageListResponse, ImageModel,
    OkResponse, RenameRequest, UploadResponse,
};
use crate::state::AppState;

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::auth::login,
        handlers::auth::cli_login,
        handlers::auth::check,
        handlers::upload::upload,
        handlers::view::view,
        handlers::view::download,
        handlers::admin::list_images,
        handlers::admin::count_images,
        handlers::admin::get_image,
        handlers::admin::rename_image,
        handlers::admin::delete_image,
        handlers::admin::restore_image,
    ),
    components(schemas(
        ImageModel,
        ImageCountResponse,
        ImageDetailResponse,
        ImageListQuery,
        ImageListResponse,
        RenameRequest,
        UploadRequest,
        UploadResponse,
        OkResponse,
        LoginRequest,
        CliLoginQuery,
        AuthSuccessResponse,
        AuthCheckResponse,
    ))
)]
struct ApiDoc;

pub fn build(state: AppState) -> Router {
    let cors = build_cors(&state);

    let vary_accept = SetResponseHeaderLayer::if_not_present(
        axum::http::header::VARY,
        HeaderValue::from_static("Accept"),
    );

    Router::new()
        // Auth
        .route("/api/auth/login", post(handlers::auth::login))
        .route("/api/auth/cli", get(handlers::auth::cli_login))
        .route("/api/auth/check", get(handlers::auth::check))
        // Upload
        .route("/api/upload", post(handlers::upload::upload))
        // Admin
        .route("/api/admin/images", get(handlers::admin::list_images))
        .route("/api/admin/images/count", get(handlers::admin::count_images))
        .route("/api/admin/images/{id}", get(handlers::admin::get_image))
        .route(
            "/api/admin/images/{id}/rename",
            post(handlers::admin::rename_image),
        )
        .route(
            "/api/admin/images/{id}/delete",
            post(handlers::admin::delete_image),
        )
        .route(
            "/api/admin/images/{id}/restore",
            post(handlers::admin::restore_image),
        )
        // OpenAPI JSON
        .route("/api/openapi.json", get(openapi_json))
        // Public view/download
        .route("/v/{*path}", get(handlers::view::view))
        .route("/d/{*path}", get(handlers::view::download))
        // Frontend: dev proxy or embedded SPA
        .fallback(crate::frontend::handler)
        .layer(cors)
        .layer(vary_accept)
        .with_state(state)
}

pub fn openapi_spec() -> UtoipaOpenApi {
    let mut spec = ApiDoc::openapi();
    let mut components = spec
        .components
        .take()
        .unwrap_or_else(|| ComponentsBuilder::new().build());
    components.add_security_scheme(
        "cookieAuth",
        SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new("ibed_token"))),
    );
    components.add_security_scheme(
        "bearerAuth",
        SecurityScheme::Http(
            HttpBuilder::new()
                .scheme(HttpAuthScheme::Bearer)
                .bearer_format("AUTH_TOKEN")
                .build(),
        ),
    );
    spec.components = Some(components);
    spec
}

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(openapi_spec())
}

fn build_cors(state: &AppState) -> CorsLayer {
    let cfg = &state.config.server;
    let is_wildcard = cfg.cors_allow_origins.iter().any(|o| o == "*");

    let layer = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .max_age(std::time::Duration::from_secs(cfg.cors_max_age));

    if is_wildcard {
        // CORS spec forbids wildcard origin with credentials
        layer.allow_origin(Any)
    } else {
        let parsed: Vec<HeaderValue> = cfg
            .cors_allow_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        layer
            .allow_credentials(true)
            .allow_origin(AllowOrigin::list(parsed))
    }
}
