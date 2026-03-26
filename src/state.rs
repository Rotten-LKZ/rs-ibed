use std::sync::Arc;

use tokio::sync::Semaphore;

use crate::auth::CliTokenStore;
use crate::config::{AppConfig, Secrets};
use crate::db::repo::ImageRepo;

#[derive(Clone)]
pub struct AppState {
    pub repo: Arc<dyn ImageRepo>,
    pub config: Arc<AppConfig>,
    pub secrets: Arc<Secrets>,
    pub cli_tokens: Arc<CliTokenStore>,
    pub workers: Arc<Semaphore>,
}
