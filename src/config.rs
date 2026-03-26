use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;

// ── Top-level ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub image: ImageConfig,
}

impl AppConfig {
    pub fn load(path: &str) -> Result<(Self, Secrets), Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&content)?;

        for (name, preset) in &config.image.presets {
            preset.validate(name)?;
        }

        let secrets =
            Secrets::from_env(&config.server.env_prefix).expect("Failed to load secrets from env");

        Ok((config, secrets))
    }
}

// ── Server ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub log_level: LogLevel,
    pub env_prefix: String,
    pub url_pattern: UrlPattern,
    pub cors_allow_origins: Vec<String>,
    pub cors_max_age: u64,
    pub enable_negotiated_cache: bool,
    pub cache_max_age: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".into(),
            port: 3000,
            log_level: LogLevel::default(),
            env_prefix: "IMG".into(),
            url_pattern: UrlPattern::default(),
            cors_allow_origins: vec!["*".into()],
            cors_max_age: 3600,
            enable_negotiated_cache: true,
            cache_max_age: 3600,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UrlPattern {
    #[default]
    Ymd,
    Ym,
    Y,
    Flat,
}

impl UrlPattern {
    pub fn date_segment(&self, year: u16, month: u8, day: u8) -> String {
        match self {
            Self::Ymd => format!("{year}/{month:02}/{day:02}"),
            Self::Ym => format!("{year}/{month:02}"),
            Self::Y => format!("{year}"),
            Self::Flat => String::new(),
        }
    }

    pub fn view_path(
        &self,
        year: u16,
        month: u8,
        day: u8,
        hash: &str,
        ext: &str,
    ) -> String {
        let seg = self.date_segment(year, month, day);
        if seg.is_empty() {
            format!("/v/{hash}.{ext}")
        } else {
            format!("/v/{seg}/{hash}.{ext}")
        }
    }

}

// ── Database ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub driver: DatabaseDriver,
    pub max_connections: u32,
    pub min_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            driver: DatabaseDriver::default(),
            max_connections: 5,
            min_connections: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseDriver {
    #[default]
    Sqlite,
    Postgres,
}

impl fmt::Display for DatabaseDriver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite => write!(f, "sqlite"),
            Self::Postgres => write!(f, "postgres"),
        }
    }
}

// ── Storage ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct StorageConfig {
    pub base_dir: String,
    pub cache_dir: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            base_dir: "./uploads".into(),
            cache_dir: "./cache".into(),
        }
    }
}

// ── Image ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ImageConfig {
    pub enable: bool,
    pub default_format: ImageFormat,
    pub quality: u8,
    pub max_workers: u32,
    pub allow_show_origin: bool,
    pub cache_ttl: u64,
    pub keep_metadata_fields: Vec<MetadataField>,
    #[serde(default)]
    pub dynamic: DynamicConfig,
    #[serde(default)]
    pub presets: HashMap<String, PresetConfig>,
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            enable: true,
            default_format: ImageFormat::default(),
            quality: 75,
            max_workers: 2,
            allow_show_origin: true,
            cache_ttl: 25200,
            keep_metadata_fields: vec![
                MetadataField::Copyright,
                MetadataField::Settings,
                MetadataField::Time,
            ],
            dynamic: DynamicConfig::default(),
            presets: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    #[default]
    Webp,
    Avif,
    Jpeg,
    Png,
    Original,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MetadataField {
    Camera,
    Settings,
    Time,
    Copyright,
    Location,
    Others,
}

impl std::str::FromStr for MetadataField {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "camera" => Ok(Self::Camera),
            "settings" => Ok(Self::Settings),
            "time" => Ok(Self::Time),
            "copyright" => Ok(Self::Copyright),
            "location" => Ok(Self::Location),
            "others" => Ok(Self::Others),
            _ => Err(format!("unknown metadata field: '{s}'")),
        }
    }
}

// ── Dynamic ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DynamicConfig {
    pub allow: bool,
    pub allow_enlargement: bool,
    pub max_width: u32,
    pub max_height: u32,
}

impl Default for DynamicConfig {
    fn default() -> Self {
        Self {
            allow: false,
            allow_enlargement: false,
            max_width: 3840,
            max_height: 2160,
        }
    }
}

// ── Presets ──────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct PresetConfig {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fit: Option<FitMode>,
    pub format: Option<ImageFormat>,
    pub quality: Option<u8>,
    #[serde(default)]
    pub eager: bool,
    pub cache_ttl: Option<u64>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FitMode {
    Cover,
    Contain,
    Fill,
    Inside,
    Outside,
}

impl PresetConfig {
    pub fn effective_fit(&self) -> FitMode {
        self.fit.unwrap_or(FitMode::Contain)
    }

    pub fn effective_format(&self, global_format: ImageFormat) -> ImageFormat {
        self.format.unwrap_or(global_format)
    }

    pub fn effective_quality(&self, global_quality: u8) -> u8 {
        self.quality.unwrap_or(global_quality)
    }

    pub fn effective_ttl(&self, global_ttl: u64) -> u64 {
        self.cache_ttl.unwrap_or(global_ttl)
    }

    pub fn validate(&self, name: &str) -> Result<(), String> {
        if self.fit.is_some() && (self.width.is_none() || self.height.is_none()) {
            return Err(format!(
                "preset \"{name}\": fit mode requires both width and height to be set"
            ));
        }
        Ok(())
    }
}

// ── Secrets ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Secrets {
    pub auth_token: String,
    pub jwt_secret: String,
    pub database_url: String,
}

impl Secrets {
    pub fn from_env(prefix: &str) -> Result<Self, String> {
        let get_var = |s: &str| {
            let key = format!("{}_{}", prefix, s);
            std::env::var(&key).map_err(|_| format!("environment variable {} is not set", key))
        };

        Ok(Self {
            auth_token: get_var("AUTH_TOKEN")?,
            jwt_secret: get_var("JWT_SECRET")?,
            database_url: get_var("DATABASE_URL")?,
        })
    }
}
