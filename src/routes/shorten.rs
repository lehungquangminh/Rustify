use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use redis::AsyncCommands;
use crate::{errors::AppError, state::AppState, utils::gen_alias};

#[derive(Deserialize)]
pub struct ShortenRequest { pub url: String, pub alias: Option<String> }

#[derive(Serialize)]
pub struct ShortenResponse { pub alias: String, pub short_url: String }

pub async fn shorten(State(state): State<AppState>, Json(req): Json<ShortenRequest>) -> Result<impl IntoResponse, AppError> {
    let url = url::Url::parse(&req.url).map_err(|_| AppError::BadRequest)?;
    let alias = match req.alias { Some(a) => a, None => gen_alias() };

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
    let _: () = r
        .set_ex(
            format!("alias:{alias}"),
            &target,
            state.cache_ttl.try_into().unwrap(),
        )
        .await?;

    let short_url = format!("{}/{}", state.base_url.trim_end_matches('/'), alias);
    Ok((axum::http::StatusCode::CREATED, Json(ShortenResponse { alias, short_url })))
}
