use std::{net::SocketAddr, time::Duration};

mod state;
mod errors;
mod utils;
mod qr;
mod cache;
mod clicks;
mod routes;

use axum::{
    routing::{get, post},
    Router,
};
use axum::{error_handling::HandleErrorLayer, BoxError};
use axum_prometheus::PrometheusMetricLayer;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use redis::aio::ConnectionManager;
use sqlx::postgres::PgPoolOptions;
use tokio::{sync::mpsc, task::JoinHandle};
use tower::{ServiceBuilder, timeout::TimeoutLayer};
use tower_governor::{GovernorLayer, key_extractor::SmartIpKeyExtractor, governor::GovernorConfigBuilder};
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
    limit::RequestBodyLimitLayer,
};
use tracing::{info};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use crate::state::AppState;
use crate::clicks::start_click_flusher;
use crate::routes::{index::index, shorten::shorten, resolve::resolve, stats::stats};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/rustify".into());
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".into());
    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into());
    let cache_ttl: usize = std::env::var("CACHE_TTL_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(600);

    let pool = PgPoolOptions::new().max_connections(8).connect(&database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let client = redis::Client::open(redis_url)?;
    let redis = ConnectionManager::new(client).await?;

    let (tx, rx) = mpsc::unbounded_channel();
    let flush = start_click_flusher(pool.clone(), rx);

    let state = AppState { pool, redis, base_url, cache_ttl, click_tx: tx };

    let (metrics_layer, metrics_handle) = PrometheusMetricLayer::pair();

    use std::sync::Arc;
    let governor_conf = GovernorConfigBuilder::default()
        .per_second(1)
        .burst_size(60)
        .key_extractor(SmartIpKeyExtractor)
        .finish()
        .expect("governor");
    let governor_conf = Arc::new(governor_conf);

    let app = Router::new()
        .route("/", get(index))
        .route("/shorten", post(shorten))
        .route("/metrics", get(|| async move { "" }))
        .route("/stats/:alias", get(stats))
        .route("/:alias", get(resolve))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|e: BoxError| async move {
                    (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        format!("middleware error: {e}"),
                    )
                }))
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
                .layer(RequestBodyLimitLayer::new(1 * 1024 * 1024))
                .layer(TimeoutLayer::new(Duration::from_secs(10)))
        )
        .route_layer(GovernorLayer { config: governor_conf.clone() })
        .layer(metrics_layer);

    let handle_clone = metrics_handle.clone();
    let app = app.route("/metrics", get(move || {
        let h = handle_clone.clone();
        async move { h.render() }
    }));

    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080);
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    info!(%addr, "listening");
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app)
        .with_graceful_shutdown(shutdown_signal(flush))
        .await?;

    Ok(())
}

async fn shutdown_signal(flush: JoinHandle<()>) {
    let _ = tokio::signal::ctrl_c().await;
    flush.abort();
}

fn init_tracing() {
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,tower_http=info,sqlx=warn"));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}
