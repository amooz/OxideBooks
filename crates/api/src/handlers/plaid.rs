use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{ExchangePlaidToken, PlaidSyncRequest, PlaidSyncResult};
use oxidebooks_db::repos::PlaidRepo;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

// ── Plaid API client (thin wrapper around the Plaid REST API) ─────────────────

const PLAID_BASE: &str = "https://production.plaid.com";
const PLAID_SANDBOX: &str = "https://sandbox.plaid.com";

fn plaid_base(state: &AppState) -> &'static str {
    if state.config.integrations.plaid_sandbox.unwrap_or(true) {
        PLAID_SANDBOX
    } else {
        PLAID_BASE
    }
}

#[derive(Serialize)]
struct LinkTokenRequest<'a> {
    client_id: &'a str,
    secret: &'a str,
    client_name: &'a str,
    country_codes: Vec<&'a str>,
    language: &'a str,
    user: LinkTokenUser<'a>,
    products: Vec<&'a str>,
}

#[derive(Serialize)]
struct LinkTokenUser<'a> {
    client_user_id: &'a str,
}

#[derive(Deserialize)]
struct LinkTokenResponse {
    link_token: String,
    expiration: String,
}

#[derive(Serialize)]
struct ExchangeRequest<'a> {
    client_id: &'a str,
    secret: &'a str,
    public_token: &'a str,
}

#[derive(Deserialize)]
struct ExchangeResponse {
    access_token: String,
    item_id: String,
}

#[derive(Serialize)]
struct TransactionsSyncRequest<'a> {
    client_id: &'a str,
    secret: &'a str,
    access_token: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<&'a str>,
    count: u32,
}

#[derive(Deserialize)]
struct TransactionsSyncResponse {
    added: Vec<PlaidTransaction>,
    next_cursor: String,
    has_more: bool,
}

#[derive(Deserialize)]
struct PlaidTransaction {
    transaction_id: String,
    date: String, // "YYYY-MM-DD"
    name: String,
    amount: f64, // Plaid: positive = debit from account
}

async fn plaid_post<B: Serialize, R: for<'de> Deserialize<'de>>(
    http: &reqwest::Client,
    url: &str,
    body: &B,
) -> Result<R, ApiError> {
    let resp = http
        .post(url)
        .json(body)
        .send()
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Plaid request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(ApiError::Internal(anyhow::anyhow!(
            "Plaid error {status}: {text}"
        )));
    }

    resp.json::<R>()
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Plaid response parse error: {e}")))
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn create_link_token(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }

    let cfg = state
        .config
        .integrations
        .plaid_client_id
        .as_deref()
        .ok_or_else(|| {
            ApiError::BadRequest("Plaid is not configured (missing PLAID_CLIENT_ID)".into())
        })?;
    let secret = state
        .config
        .integrations
        .plaid_secret
        .as_deref()
        .ok_or_else(|| {
            ApiError::BadRequest("Plaid is not configured (missing PLAID_SECRET)".into())
        })?;

    let url = format!("{}/link/token/create", plaid_base(&state));
    let body = LinkTokenRequest {
        client_id: cfg,
        secret,
        client_name: "OxideBooks",
        country_codes: vec!["US", "CA", "GB"],
        language: "en",
        user: LinkTokenUser {
            client_user_id: &claims.sub,
        },
        products: vec!["transactions"],
    };

    let http = reqwest::Client::new();
    let resp: LinkTokenResponse = plaid_post(&http, &url, &body).await?;

    Ok(Json(serde_json::json!({
        "data": {
            "link_token": resp.link_token,
            "expiration": resp.expiration,
        }
    })))
}

pub async fn exchange_public_token(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<ExchangePlaidToken>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }

    let client_id = state
        .config
        .integrations
        .plaid_client_id
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("Plaid not configured".into()))?;
    let secret = state
        .config
        .integrations
        .plaid_secret
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("Plaid not configured".into()))?;

    let url = format!("{}/item/public_token/exchange", plaid_base(&state));
    let req = ExchangeRequest {
        client_id,
        secret,
        public_token: &body.public_token,
    };

    let http = reqwest::Client::new();
    let resp: ExchangeResponse = plaid_post(&http, &url, &req).await?;

    let item = PlaidRepo::create_item(
        &state.db,
        &claims.org,
        &body.bank_account_id,
        &resp.item_id,
        &resp.access_token,
        body.institution_id.as_deref(),
        body.institution_name.as_deref(),
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": item })),
    ))
}

pub async fn list_items(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    let items = PlaidRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": items })))
}

pub async fn disconnect_item(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    PlaidRepo::disconnect(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn sync_transactions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<PlaidSyncRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }

    let client_id = state
        .config
        .integrations
        .plaid_client_id
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("Plaid not configured".into()))?;
    let secret = state
        .config
        .integrations
        .plaid_secret
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("Plaid not configured".into()))?;

    let items = PlaidRepo::list_active(&state.db, &claims.org).await?;
    let items: Vec<_> = if let Some(ref filter_item_id) = body.item_id {
        items
            .into_iter()
            .filter(|i| &i.item_id == filter_item_id)
            .collect()
    } else {
        items
    };

    let http = reqwest::Client::new();
    let mut total_added = 0usize;
    let mut total_skipped = 0usize;
    let items_synced = items.len();

    for item in &items {
        let mut cursor = item.cursor.clone();

        loop {
            let req = TransactionsSyncRequest {
                client_id,
                secret,
                access_token: &item.access_token,
                cursor: cursor.as_deref(),
                count: 500,
            };

            let url = format!("{}/transactions/sync", plaid_base(&state));
            let resp: TransactionsSyncResponse = plaid_post(&http, &url, &req).await?;

            for txn in &resp.added {
                let date = parse_plaid_date(&txn.date)?;
                // Plaid: positive amount = debit (money out), negative = credit (money in)
                let (amount_minor, txn_type) = if txn.amount >= 0.0 {
                    ((txn.amount * 100.0).round() as i64, "debit")
                } else {
                    ((-txn.amount * 100.0).round() as i64, "credit")
                };

                let inserted = PlaidRepo::upsert_feed_txn(
                    &state.db,
                    Uuid::parse_str(&item.organization_id.to_string()).unwrap_or_default(),
                    Uuid::parse_str(&item.bank_account_id.to_string()).unwrap_or_default(),
                    &txn.transaction_id,
                    date,
                    &txn.name,
                    amount_minor,
                    txn_type,
                )
                .await?;

                if inserted {
                    total_added += 1;
                } else {
                    total_skipped += 1;
                }
            }

            cursor = Some(resp.next_cursor.clone());

            if !resp.has_more {
                PlaidRepo::update_cursor(&state.db, item.id, &resp.next_cursor).await?;
                break;
            }
        }
    }

    let result = PlaidSyncResult {
        items_synced,
        transactions_added: total_added,
        transactions_skipped: total_skipped,
    };
    Ok(Json(serde_json::json!({ "data": result })))
}

fn parse_plaid_date(s: &str) -> Result<time::Date, ApiError> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return Err(ApiError::BadRequest(format!("invalid Plaid date: {s}")));
    }
    let y: i32 = parts[0]
        .parse()
        .map_err(|_| ApiError::BadRequest(format!("invalid date: {s}")))?;
    let m: u8 = parts[1]
        .parse()
        .map_err(|_| ApiError::BadRequest(format!("invalid date: {s}")))?;
    let d: u8 = parts[2]
        .parse()
        .map_err(|_| ApiError::BadRequest(format!("invalid date: {s}")))?;
    let month = time::Month::try_from(m)
        .map_err(|_| ApiError::BadRequest(format!("invalid month in date: {s}")))?;
    time::Date::from_calendar_date(y, month, d)
        .map_err(|_| ApiError::BadRequest(format!("invalid date: {s}")))
}
