use axum::{http::StatusCode, response::{IntoResponse, Response}};

#[derive(thiserror::Error, Debug)]
pub enum AppError {
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
