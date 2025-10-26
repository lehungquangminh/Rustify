use std::{net::SocketAddr, time::Duration};

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Json, Router,
};
use axum_prometheus::PrometheusMetricLayer;
use opentelemetry::sdk::{self, propagation::TraceContextPropagator};
use qrcode::QrCode;
use rand::{distributions::Alphanumeric, Rng};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::{sync::mpsc, task::JoinHandle};
use tower::{ServiceBuilder, timeout::TimeoutLayer};
use tower_governor::{GovernorLayer, key_extractor::SmartIpKeyExtractor, governor::GovernorConfigBuilder};
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing::{error, info, instrument};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    redis: ConnectionManager,
    base_url: String,
    cache_ttl: usize,
    click_tx: mpsc::UnboundedSender<String>,
}

async fn index() -> impl IntoResponse {
    Html(include_str!("../static/index.html"))
}

#[derive(Deserialize)]
struct ShortenRequest { url: String, alias: Option<String> }

#[derive(Serialize)]
struct ShortenResponse { alias: String, short_url: String }

#[derive(Serialize)]
struct StatsResponse { alias: String, url: String, clicks: i64 }

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

    let metrics = PrometheusMetricLayer::new();

    let governor_conf = GovernorConfigBuilder::default()
        .per_second(1)
        .burst_size(60)
        .key_extractor(SmartIpKeyExtractor)
        .finish()
        .expect("governor");

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
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
                .layer(TimeoutLayer::new(Duration::from_secs(10)))
                .layer(GovernorLayer::new(&governor_conf))
        )
        .layer(metrics);

    let recorder_handle = metrics.handle();
    let app = app.route("/metrics", get(move || {
        let h = recorder_handle.clone();
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

fn gen_alias() -> String {
    let s: String = rand::thread_rng().sample_iter(&Alphanumeric).take(7).map(char::from).collect();
    base62::encode(s)
}

#[instrument(skip(state))]
async fn shorten(State(state): State<AppState>, Json(req): Json<ShortenRequest>) -> Result<impl IntoResponse, AppError> {
    use sqlx::Row;
    let url = url::Url::parse(&req.url).map_err(|_| AppError::BadRequest)?;
    let alias = match req.alias {
        Some(a) => a,
        None => gen_alias(),
    };

    let rec = sqlx::query(
        "INSERT INTO links(alias, url) VALUES ($1,$2) ON CONFLICT DO NOTHING RETURNING alias, url",
    )
    .bind(&alias)
    .bind(url.as_str())
    .fetch_optional(&state.pool)
    .await?;

    let (alias, target) = if let Some(row) = rec {
        let alias: String = row.try_get("alias").unwrap_or_default();
        let url: String = row.try_get("url").unwrap_or_default();
        (alias, url)
    } else {
        let e = sqlx::query("SELECT alias, url FROM links WHERE alias = $1")
            .bind(&alias)
            .fetch_optional(&state.pool)
            .await?;
        if let Some(row) = e {
            (row.try_get("alias").unwrap_or(alias), row.try_get("url").unwrap())
        } else { return Err(AppError::Conflict); }
    };

    let mut r = state.redis.clone();
    let _: () = r.set_ex(format!("alias:{alias}"), &target, state.cache_ttl).await?;

    let short_url = format!("{}/{}", state.base_url.trim_end_matches('/'), alias);
    Ok((StatusCode::CREATED, Json(ShortenResponse { alias, short_url })))
}

#[instrument(skip(state, headers))]
async fn resolve(State(state): State<AppState>, Path(alias): Path<String>, headers: HeaderMap) -> Result<impl IntoResponse, AppError> {
    let target = get_target(&state, &alias).await?;
    let _ = state.click_tx.send(alias);
    let mut resp = Redirect::temporary(&target).into_response();
    if let Some(v) = headers.get("accept") { if v == "image/png" { if let Ok(img) = qr_png(&target) { resp = ([("content-type","image/png")], img).into_response(); } } }
    Ok(resp)
}

#[instrument(skip(state))]
async fn stats(State(state): State<AppState>, Path(alias): Path<String>) -> Result<impl IntoResponse, AppError> {
    use sqlx::Row;
    let rec = sqlx::query("SELECT url FROM links WHERE alias = $1")
        .bind(&alias)
        .fetch_optional(&state.pool)
        .await?;
    let url: String = if let Some(r) = rec { r.try_get("url").unwrap() } else { return Err(AppError::NotFound) };
    let clicks: i64 = sqlx::query("SELECT COALESCE(SUM(n),0) AS clicks FROM clicks WHERE alias = $1")
        .bind(&alias)
        .fetch_one(&state.pool)
        .await?
        .try_get("clicks")?;
    Ok(Json(StatsResponse { alias, url, clicks }))
}

async fn get_target(state: &AppState, alias: &str) -> Result<String, AppError> {
    let mut r = state.redis.clone();
    if let Ok(Some(v)) = r.get::<_, Option<String>>(format!("alias:{alias}")).await { return Ok(v); }
    use sqlx::Row;
    let rec = sqlx::query("SELECT url FROM links WHERE alias = $1")
        .bind(alias)
        .fetch_optional(&state.pool)
        .await?;
    let url: String = if let Some(r) = rec { r.try_get("url").unwrap() } else { return Err(AppError::NotFound) };
    let _: () = r.set_ex(format!("alias:{alias}"), &url, state.cache_ttl).await.unwrap_or(());
    Ok(url)
}

fn qr_png(data: &str) -> anyhow::Result<Vec<u8>> {
    let code = QrCode::new(data.as_bytes())?;
    let img = code.render::<image::Luma<u8>>().build();
    let mut buf = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buf);
    let dyn_img = image::DynamicImage::ImageLuma8(img);
    dyn_img.write_to(&mut cursor, image::ImageFormat::Png)?;
    Ok(buf)
}

fn start_click_flusher(pool: PgPool, mut rx: mpsc::UnboundedReceiver<String>) -> JoinHandle<()> {
    tokio::spawn(async move {
        use std::collections::HashMap;
        let mut counts: HashMap<String, i64> = HashMap::new();
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        loop {
            tokio::select! {
                _ = tick.tick() => {
                    if counts.is_empty() { continue; }
                    let data: Vec<(String, i64)> = counts.drain().collect();
                    let mut tx = match pool.begin().await { Ok(t) => t, Err(e) => { error!(?e, "tx"); continue; } };
                    for (alias, n) in data {
                        let _ = sqlx::query("INSERT INTO clicks(alias, ts, n) VALUES ($1, now(), $2)")
                            .bind(&alias)
                            .bind(n)
                            .execute(&mut tx)
                            .await;
                    }
                    let _ = tx.commit().await;
                }
                msg = rx.recv() => {
                    if let Some(a) = msg { *counts.entry(a).or_default() += 1; }
                    else { break; }
                }
            }
        }
    })
}

#[derive(thiserror::Error, Debug)]
enum AppError {
    #[error("bad request")] BadRequest,
    #[error("conflict")] Conflict,
    #[error("not found")] NotFound,
    #[error(transparent)] Anyhow(#[from] anyhow::Error),
    #[error(transparent)] Sqlx(#[from] sqlx::Error),
    #[error(transparent)] Redis(#[from] redis::RedisError),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let code = match self {
            AppError::BadRequest => StatusCode::BAD_REQUEST,
            AppError::Conflict => StatusCode::CONFLICT,
            AppError::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (code, self.to_string()).into_response()
    }
}
