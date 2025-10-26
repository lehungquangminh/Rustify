use axum::{extract::{Path, State}, response::{IntoResponse, Redirect}, http::HeaderMap};
use crate::{errors::AppError, state::AppState, cache::get_target, qr::qr_png};
use tracing::instrument;

#[instrument(skip(state, headers))]
pub async fn resolve(State(state): State<AppState>, Path(alias): Path<String>, headers: HeaderMap) -> Result<impl IntoResponse, AppError> {
    let target = get_target(&state, &alias).await?;
    let _ = state.click_tx.send(alias);
    let mut resp = Redirect::temporary(&target).into_response();
    if let Some(v) = headers.get("accept") {
        if v == "image/png" {
            if let Ok(img) = qr_png(&target) {
                resp = ([("content-type","image/png")], img).into_response();
            }
        }
    }
    Ok(resp)
}
