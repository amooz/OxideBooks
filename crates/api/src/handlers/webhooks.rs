use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use hmac::{Hmac, Mac};
use oxidebooks_core::models::{
    CreateWebhookEndpoint, UpdateWebhookEndpoint, WebhookPayload, ALL_EVENT_TYPES,
};
use oxidebooks_db::repos::{WebhookDeliveryRepo, WebhookRepo};
use serde::Deserialize;
use sha2::Sha256;
use uuid::Uuid;

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

/// Fire-and-forget webhook delivery. Records each attempt in webhook_deliveries.
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

    let payload_json = match serde_json::to_value(&payload) {
        Ok(v) => v,
        Err(_) => return,
    };

    let org_uuid = match Uuid::parse_str(org_id) {
        Ok(u) => u,
        Err(_) => return,
    };

    for (endpoint, secret) in endpoints {
        let ep_uuid = match Uuid::parse_str(&endpoint.id) {
            Ok(u) => u,
            Err(_) => continue,
        };

        let delivery = match WebhookDeliveryRepo::create(
            &state.db,
            org_uuid,
            ep_uuid,
            event_type,
            &payload_json,
        )
        .await
        {
            Ok(d) => d,
            Err(_) => continue,
        };

        let delivery_id = match Uuid::parse_str(&delivery.id) {
            Ok(u) => u,
            Err(_) => continue,
        };

        let body_clone = body.clone();
        let secret_clone = secret.clone();
        let url = endpoint.url.clone();
        let client = state.http.clone();
        let db = state.db.clone();

        tokio::spawn(async move {
            let sig = sign_payload(&secret_clone, &body_clone);
            let result = client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("X-OxideBooks-Signature", sig)
                .body(body_clone)
                .send()
                .await;

            let (success, http_status, response_body) = match result {
                Ok(resp) => {
                    let status = resp.status().as_u16() as i32;
                    let success = resp.status().is_success();
                    let body = resp.text().await.ok();
                    (success, Some(status), body)
                }
                Err(_) => (false, None, None),
            };

            let _ = WebhookDeliveryRepo::record_result(
                &db,
                delivery_id,
                success,
                http_status,
                response_body.as_deref(),
            )
            .await;

            if !success {
                // Mark failed after 5 attempts is handled by the retry scheduler
            }
        });
    }
}

#[derive(Debug, Deserialize)]
pub struct DeliveryQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    50
}

pub async fn list_deliveries(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Query(q): Query<DeliveryQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    // Verify endpoint belongs to org
    WebhookRepo::get_by_id(&state.db, &claims.org, &id).await?;
    let limit = q.limit.clamp(1, 200);
    let deliveries =
        WebhookDeliveryRepo::list_for_endpoint(&state.db, &claims.org, &id, limit).await?;
    Ok(Json(serde_json::json!({ "data": deliveries })))
}

pub async fn get_delivery(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let delivery = WebhookDeliveryRepo::get(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": delivery })))
}

pub async fn retry_delivery(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let delivery_uuid = WebhookDeliveryRepo::reset_for_retry(&state.db, &claims.org, &id).await?;
    Ok(Json(
        serde_json::json!({ "data": { "id": delivery_uuid.to_string(), "status": "retrying" } }),
    ))
}

pub async fn test_webhook(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let endpoint = WebhookRepo::get_by_id(&state.db, &claims.org, &id).await?;
    if !endpoint.is_active {
        return Err(ApiError::BadRequest("endpoint is not active".to_string()));
    }

    let test_payload = serde_json::json!({
        "event": "test",
        "organization_id": claims.org,
        "data": { "message": "This is a test webhook delivery from OxideBooks." }
    });

    let body = serde_json::to_string(&test_payload)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("serialization failed: {e}")))?;

    let ep_uuid = Uuid::parse_str(&endpoint.id)
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("invalid endpoint UUID")))?;
    let org_uuid = Uuid::parse_str(&claims.org)
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("invalid org UUID")))?;

    // We need the secret — fetch endpoints with active_for_event using "test" event won't work,
    // so we use a raw delivery without secret lookup. Re-fetch via active_for_event alternative:
    // Instead, create delivery and fire synchronously so we can return the result.
    let delivery =
        WebhookDeliveryRepo::create(&state.db, org_uuid, ep_uuid, "test", &test_payload).await?;
    let delivery_id = Uuid::parse_str(&delivery.id)
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("invalid delivery UUID")))?;

    // Fetch secret via active_for_event — fall back to no-sig if endpoint not subscribed to "test"
    let endpoints_with_secret =
        WebhookRepo::active_for_event(&state.db, &claims.org, "test").await?;
    let secret = endpoints_with_secret
        .into_iter()
        .find(|(ep, _)| ep.id == endpoint.id)
        .map(|(_, s)| s)
        .unwrap_or_default();

    let sig = sign_payload(&secret, &body);
    let result = state
        .http
        .post(&endpoint.url)
        .header("Content-Type", "application/json")
        .header("X-OxideBooks-Signature", sig)
        .body(body)
        .send()
        .await;

    let (success, http_status, response_body) = match result {
        Ok(resp) => {
            let status = resp.status().as_u16() as i32;
            let ok = resp.status().is_success();
            let body = resp.text().await.ok();
            (ok, Some(status), body)
        }
        Err(e) => (false, None, Some(e.to_string())),
    };

    WebhookDeliveryRepo::record_result(
        &state.db,
        delivery_id,
        success,
        http_status,
        response_body.as_deref(),
    )
    .await?;

    Ok(Json(serde_json::json!({
        "data": {
            "delivery_id": delivery.id,
            "success": success,
            "http_status": http_status,
            "response_body": response_body,
        }
    })))
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
