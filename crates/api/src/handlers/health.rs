use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

use crate::state::AppState;

/// GET /health/live — always 200 if the process is running.
pub async fn liveness() -> &'static str {
    "ok"
}

/// GET /health/ready — probes the database; 503 if unreachable.
pub async fn readiness(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();
    match sqlx::query("SELECT 1").execute(&state.db).await {
        Ok(_) => {
            let latency_ms = start.elapsed().as_millis();
            (
                StatusCode::OK,
                Json(json!({
                    "status": "ok",
                    "checks": {
                        "database": { "status": "ok", "latency_ms": latency_ms }
                    }
                })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "degraded",
                "checks": {
                    "database": { "status": "error", "error": e.to_string() }
                }
            })),
        )
            .into_response(),
    }
}
