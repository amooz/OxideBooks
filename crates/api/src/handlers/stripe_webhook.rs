use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::{error::ApiResult, state::AppState};

type HmacSha256 = Hmac<Sha256>;

fn verify_stripe_signature(payload: &[u8], sig_header: &str, secret: &str) -> bool {
    // Stripe sends: t=<timestamp>,v1=<hex_sig>[,v0=...]
    let timestamp = sig_header
        .split(',')
        .find(|p| p.starts_with("t="))
        .and_then(|p| p.strip_prefix("t="));
    let v1 = sig_header
        .split(',')
        .find(|p| p.starts_with("v1="))
        .and_then(|p| p.strip_prefix("v1="));

    let (Some(ts), Some(expected_hex)) = (timestamp, v1) else {
        return false;
    };

    let signed_payload = format!("{}.{}", ts, String::from_utf8_lossy(payload));
    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(signed_payload.as_bytes());
    let computed = mac.finalize().into_bytes();
    let computed_hex: String = computed.iter().map(|b| format!("{b:02x}")).collect();
    computed_hex == expected_hex
}

pub async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<StatusCode> {
    let sig_header = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let secret = state
        .config
        .app
        .stripe_webhook_secret
        .as_deref()
        .unwrap_or("");

    if !secret.is_empty() && !verify_stripe_signature(&body, sig_header, secret) {
        return Ok(StatusCode::UNAUTHORIZED);
    }

    let Ok(event) = serde_json::from_slice::<serde_json::Value>(&body) else {
        return Ok(StatusCode::BAD_REQUEST);
    };

    let event_type = event["type"].as_str().unwrap_or("");

    if event_type == "payment_intent.succeeded" {
        let metadata = &event["data"]["object"]["metadata"];
        if let Some(token) = metadata["payment_link_token"].as_str() {
            let _ = oxidebooks_db::repos::PaymentLinkRepo::mark_paid(&state.db, token).await;
        }
    }

    Ok(StatusCode::OK)
}
