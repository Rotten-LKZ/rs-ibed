use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{HeaderMap, header};
use axum_extra::extract::CookieJar;
use jsonwebtoken::crypto::{self, CryptoProvider};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::config::Secrets;
use crate::error::AppError;
use crate::state::AppState;

const JWT_COOKIE_NAME: &str = "ibed_token";
const JWT_EXPIRY_SECS: u64 = 7 * 24 * 3600; // 7 days
const CLI_TOKEN_TTL: Duration = Duration::from_secs(180); // 3 minutes

pub fn install_crypto_provider() {
    let provider: &'static CryptoProvider = &crypto::rust_crypto::DEFAULT_PROVIDER;
    let _ = provider.install_default();
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: u64,
    pub iat: u64,
}

/// In-memory store for short-lived CLI login tokens.
pub struct CliTokenStore {
    tokens: Mutex<HashMap<String, Instant>>,
}

impl CliTokenStore {
    pub fn new() -> Self {
        Self {
            tokens: Mutex::new(HashMap::new()),
        }
    }

    /// Generate a new random token, store it, and return the token string.
    pub fn generate(&self) -> String {
        let mut rng = rand::rng();
        let token: String = (0..48)
            .map(|_| {
                let idx = rng.random_range(0..36u8);
                if idx < 10 {
                    (b'0' + idx) as char
                } else {
                    (b'a' + idx - 10) as char
                }
            })
            .collect();

        let mut map = self.tokens.lock().unwrap();
        // Clean up expired tokens while we're at it
        map.retain(|_, created| created.elapsed() < CLI_TOKEN_TTL);
        map.insert(token.clone(), Instant::now());
        token
    }

    /// Validate and consume a CLI token (one-time use).
    pub fn validate_and_consume(&self, token: &str) -> bool {
        let mut map = self.tokens.lock().unwrap();
        if let Some(created) = map.remove(token) {
            created.elapsed() < CLI_TOKEN_TTL
        } else {
            false
        }
    }
}

/// Create a signed JWT token.
pub fn create_jwt(secret: &str) -> Result<String, AppError> {
    let now = chrono::Utc::now().timestamp() as u64;
    let claims = Claims {
        sub: "admin".to_string(),
        iat: now,
        exp: now + JWT_EXPIRY_SECS,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(e.to_string()))
}

/// Validate a JWT token string.
pub fn validate_jwt(token: &str, secret: &str) -> Result<Claims, AppError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| AppError::Unauthorized)
}

/// Build a Set-Cookie header value for the JWT.
pub fn jwt_cookie_header(jwt: &str) -> String {
    format!(
        "{JWT_COOKIE_NAME}={jwt}; HttpOnly; SameSite=Lax; Path=/; Max-Age={JWT_EXPIRY_SECS}"
    )
}

/// Verify authentication from request headers.
///
/// Checks in order:
/// 1. `Authorization: Bearer {AUTH_TOKEN}` — direct token match
/// 2. `ibed_token` cookie — JWT validation
pub fn verify_auth(headers: &HeaderMap, secrets: &Secrets) -> Result<(), AppError> {
    // 1. Bearer token
    if let Some(value) = headers.get(header::AUTHORIZATION) {
        if let Ok(s) = value.to_str() {
            if let Some(token) = s.strip_prefix("Bearer ") {
                if token == secrets.auth_token {
                    return Ok(());
                }
            }
        }
    }

    // 2. JWT cookie
    let jar = CookieJar::from_headers(headers);
    let cookie = jar
        .get(JWT_COOKIE_NAME)
        .ok_or(AppError::Unauthorized)?;

    validate_jwt(cookie.value(), &secrets.jwt_secret)?;
    Ok(())
}

/// Extractor: requires authentication via Bearer token or JWT cookie.
/// Usage: put `AuthUser` in handler params to require authentication.
pub struct AuthUser;

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        verify_auth(&parts.headers, &state.secrets)?;
        Ok(AuthUser)
    }
}
