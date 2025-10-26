use axum::response::{Html, IntoResponse};

pub async fn index() -> impl IntoResponse {
    Html(include_str!("../../static/index.html"))
}
