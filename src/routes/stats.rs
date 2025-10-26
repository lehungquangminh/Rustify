use axum::{extract::{Path, State}, response::IntoResponse, Json};
use serde::Serialize;
use sqlx::Row;
use crate::{errors::AppError, state::AppState};
use tracing::instrument;

#[derive(Serialize)]
pub struct StatsResponse { pub alias: String, pub url: String, pub clicks: i64 }

#[instrument(skip(state))]
pub async fn stats(State(state): State<AppState>, Path(alias): Path<String>) -> Result<impl IntoResponse, AppError> {
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
