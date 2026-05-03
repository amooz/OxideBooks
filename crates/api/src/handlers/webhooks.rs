use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use hmac::{Hmac, Mac};
use oxidebooks_core::models::{
    CreateWebhookEndpoint, UpdateWebhookEndpoint, WebhookPayload, ALL_EVENT_TYPES,
};
use oxidebooks_db::repos::WebhookRepo;
use sha2::Sha256;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

pub async fn list_webhooks(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let endpoints = WebhookRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": endpoints })))
}

pub async fn get_webhook(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let endpoint = WebhookRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(endpoint)))
}

pub async fn create_webhook(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateWebhookEndpoint>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    // Validate event types
    for event in &body.events {
        if !ALL_EVENT_TYPES.contains(&event.as_str()) {
            return Err(ApiError::BadRequest(format!("unknown event type: {event}")));
        }
    }
    let (endpoint, secret) = WebhookRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "endpoint": endpoint, "secret": secret })),
    ))
}

pub async fn update_webhook(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateWebhookEndpoint>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    if let Some(events) = &body.events {
        for event in events {
            if !ALL_EVENT_TYPES.contains(&event.as_str()) {
                return Err(ApiError::BadRequest(format!("unknown event type: {event}")));
            }
        }
    }
    let endpoint = WebhookRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!(endpoint)))
}

pub async fn delete_webhook(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    WebhookRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Fire-and-forget webhook delivery. Called from mutating handlers after successful DB writes.
pub async fn deliver_event(
    state: &AppState,
    org_id: &str,
    event_type: &str,
    data: serde_json::Value,
) {
    let endpoints = match WebhookRepo::active_for_event(&state.db, org_id, event_type).await {
        Ok(e) => e,
        Err(_) => return,
    };

    let payload = WebhookPayload {
        event: event_type.to_string(),
        organization_id: org_id.to_string(),
        data,
    };

    let body = match serde_json::to_string(&payload) {
        Ok(b) => b,
        Err(_) => return,
    };

    for (endpoint, secret) in endpoints {
        let body_clone = body.clone();
        let secret_clone = secret.clone();
        let url = endpoint.url.clone();
        let client = state.http.clone();

        tokio::spawn(async move {
            let sig = sign_payload(&secret_clone, &body_clone);
            let _ = client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("X-OxideBooks-Signature", sig)
                .body(body_clone)
                .send()
                .await;
        });
    }
}

fn sign_payload(secret: &str, body: &str) -> String {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key size");
    mac.update(body.as_bytes());
    let result = mac.finalize().into_bytes();
    format!("sha256={}", hex_encode(&result))
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
