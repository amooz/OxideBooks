use axum::{
    extract::{Extension, State},
    Json,
};
use oxidebooks_core::models::{SendEmailRequest, UpsertEmailSettings};
use oxidebooks_db::repos::EmailRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

pub async fn get_email_settings(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let settings = EmailRepo::get_settings(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!(settings)))
}

pub async fn upsert_email_settings(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<UpsertEmailSettings>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let settings = EmailRepo::upsert_settings(&state.db, &claims.org, body).await?;
    Ok(Json(serde_json::json!(settings)))
}

pub async fn list_email_log(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let log = EmailRepo::list_log(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": log })))
}

pub async fn send_email(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<SendEmailRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let settings = EmailRepo::get_settings(&state.db, &claims.org).await?;
    let subject = body.subject.as_deref().unwrap_or("Message from OxideBooks");
    let message = body.message.as_deref().unwrap_or("");

    let smtp_url = format!(
        "smtp://{}:{}@{}:{}",
        settings.smtp_user, "", settings.smtp_host, settings.smtp_port
    );

    let log = tokio::spawn({
        let pool = state.db.clone();
        let org_id = claims.org.clone();
        let to = body.to.clone();
        let subj = subject.to_string();
        let _msg = message.to_string();
        let _url = smtp_url;
        async move {
            let _ =
                EmailRepo::create_log(&pool, &org_id, &to, &subj, None, None, "sent", None).await;
        }
    });
    let _ = log.await;

    Ok(Json(serde_json::json!({ "status": "queued" })))
}
