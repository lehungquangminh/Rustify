use crate::{errors::AppError, state::AppState};
use redis::AsyncCommands;
use sqlx::Row;

pub async fn get_target(state: &AppState, alias: &str) -> Result<String, AppError> {
    let mut r = state.redis.clone();
    if let Ok(Some(v)) = r.get::<_, Option<String>>(format!("alias:{alias}")).await {
        return Ok(v);
    }
    let rec = sqlx::query("SELECT url FROM links WHERE alias = $1")
        .bind(alias)
        .fetch_optional(&state.pool)
        .await?;
    let url: String = if let Some(r) = rec { r.try_get("url").unwrap() } else { return Err(AppError::NotFound) };
    let _: () = r.set_ex(format!("alias:{alias}"), &url, state.cache_ttl).await.unwrap_or(());
    Ok(url)
}
