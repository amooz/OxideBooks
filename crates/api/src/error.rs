use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("not found")]
    NotFound,

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden")]
    Forbidden,

    #[error("{0}")]
    BadRequest(String),

    #[error("{0}")]
    Conflict(String),

    #[error("internal server error")]
    Internal(#[from] anyhow::Error),

    #[error("database error")]
    Db(#[from] oxidebooks_db::DbError),

    #[error("validation error: {0}")]
    Validation(#[from] oxidebooks_core::CoreError),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, code, message) = match &self {
            ApiError::NotFound => (StatusCode::NOT_FOUND, "not_found", self.to_string()),
            ApiError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, "unauthorized", self.to_string())
            }
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "forbidden", self.to_string()),
            ApiError::BadRequest(msg) => {
                (StatusCode::BAD_REQUEST, "bad_request", msg.clone())
            }
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, "conflict", msg.clone()),
            ApiError::Validation(e) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "validation_error", e.to_string())
            }
            ApiError::Db(e) if e.is_not_found() => {
                (StatusCode::NOT_FOUND, "not_found", "record not found".into())
            }
            ApiError::Db(e) if e.is_conflict() => {
                (StatusCode::CONFLICT, "conflict", e.to_string())
            }
            ApiError::Db(_) | ApiError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "an internal error occurred".into(),
            ),
        };

        (status, Json(json!({ "error": { "code": code, "message": message } }))).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
