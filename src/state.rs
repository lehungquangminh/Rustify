use tokio::sync::mpsc;
use sqlx::PgPool;
use redis::aio::ConnectionManager;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub redis: ConnectionManager,
    pub base_url: String,
    pub cache_ttl: usize,
    pub click_tx: mpsc::UnboundedSender<String>,
}
