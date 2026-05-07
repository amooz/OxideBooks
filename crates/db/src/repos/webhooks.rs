use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use oxidebooks_core::models::{CreateWebhookEndpoint, UpdateWebhookEndpoint, WebhookEndpoint};
use rand::RngCore;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct WebhookRow {
    id: Uuid,
    organization_id: Uuid,
    url: String,
    secret: String,
    events: serde_json::Value,
    is_active: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn from_row(r: WebhookRow) -> WebhookEndpoint {
    let events: Vec<String> = serde_json::from_value(r.events).unwrap_or_default();
    WebhookEndpoint {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        url: r.url,
        events,
        is_active: r.is_active,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

fn generate_secret() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

const COLS: &str = "id, organization_id, url, secret, events, is_active, created_at, updated_at";

pub struct WebhookRepo;

impl WebhookRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<WebhookEndpoint>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<WebhookRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM webhook_endpoints WHERE organization_id = $1 ORDER BY created_at"
        ))
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<WebhookEndpoint, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: WebhookRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM webhook_endpoints WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(from_row(row))
    }

    /// Returns (endpoint, secret) — secret is only exposed on creation.
    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateWebhookEndpoint,
    ) -> Result<(WebhookEndpoint, String), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let secret = generate_secret();
        let events_json =
            serde_json::to_value(&input.events).map_err(|e| DbError::Internal(e.to_string()))?;

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO webhook_endpoints (organization_id, url, secret, events, is_active) \
             VALUES ($1,$2,$3,$4,$5) RETURNING id",
        )
        .bind(org_uuid)
        .bind(&input.url)
        .bind(&secret)
        .bind(events_json)
        .bind(input.is_active)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let endpoint = Self::get_by_id(pool, org_id, &id.to_string()).await?;
        Ok((endpoint, secret))
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateWebhookEndpoint,
    ) -> Result<WebhookEndpoint, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let events_json = input
            .events
            .as_ref()
            .map(|e| serde_json::to_value(e).map_err(|e| DbError::Internal(e.to_string())))
            .transpose()?;

        let n = sqlx::query(
            "UPDATE webhook_endpoints SET \
             url       = COALESCE($1, url), \
             events    = COALESCE($2, events), \
             is_active = COALESCE($3, is_active), \
             updated_at = NOW() \
             WHERE id = $4 AND organization_id = $5",
        )
        .bind(input.url)
        .bind(events_json)
        .bind(input.is_active)
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if n == 0 {
            return Err(DbError::NotFound);
        }
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n = sqlx::query("DELETE FROM webhook_endpoints WHERE id = $1 AND organization_id = $2")
            .bind(id_uuid)
            .bind(org_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?
            .rows_affected();
        if n == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    /// Fetch all active endpoints subscribed to the given event type.
    pub async fn active_for_event(
        pool: &PgPool,
        org_id: &str,
        event_type: &str,
    ) -> Result<Vec<(WebhookEndpoint, String)>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<WebhookRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM webhook_endpoints \
             WHERE organization_id = $1 AND is_active = TRUE \
               AND events @> $2::jsonb"
        ))
        .bind(org_uuid)
        .bind(serde_json::json!([event_type]))
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let secret = r.secret.clone();
                (from_row(r), secret)
            })
            .collect())
    }
}

// ── Delivery tracking ─────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WebhookDelivery {
    pub id: String,
    pub organization_id: String,
    pub endpoint_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub http_status: Option<i32>,
    pub response_body: Option<String>,
    pub attempt_count: i32,
    #[serde(with = "time::serde::rfc3339::option")]
    pub next_retry_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub delivered_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct DeliveryRow {
    id: Uuid,
    organization_id: Uuid,
    endpoint_id: Uuid,
    event_type: String,
    payload: serde_json::Value,
    status: String,
    http_status: Option<i32>,
    response_body: Option<String>,
    attempt_count: i32,
    next_retry_at: Option<OffsetDateTime>,
    delivered_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
}

impl From<DeliveryRow> for WebhookDelivery {
    fn from(r: DeliveryRow) -> Self {
        WebhookDelivery {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            endpoint_id: r.endpoint_id.to_string(),
            event_type: r.event_type,
            payload: r.payload,
            status: r.status,
            http_status: r.http_status,
            response_body: r.response_body,
            attempt_count: r.attempt_count,
            next_retry_at: r.next_retry_at,
            delivered_at: r.delivered_at,
            created_at: r.created_at,
        }
    }
}

const DEL_COLS: &str =
    "id, organization_id, endpoint_id, event_type, payload, status, http_status, \
     response_body, attempt_count, next_retry_at, delivered_at, created_at";

pub struct WebhookDeliveryRepo;

impl WebhookDeliveryRepo {
    pub async fn create(
        pool: &PgPool,
        org_id: Uuid,
        endpoint_id: Uuid,
        event_type: &str,
        payload: &serde_json::Value,
    ) -> Result<WebhookDelivery, DbError> {
        let row: DeliveryRow = sqlx::query_as(&format!(
            "INSERT INTO webhook_deliveries \
             (organization_id, endpoint_id, event_type, payload) \
             VALUES ($1,$2,$3,$4) RETURNING {DEL_COLS}"
        ))
        .bind(org_id)
        .bind(endpoint_id)
        .bind(event_type)
        .bind(payload)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.into())
    }

    pub async fn record_result(
        pool: &PgPool,
        id: Uuid,
        success: bool,
        http_status: Option<i32>,
        response_body: Option<&str>,
    ) -> Result<(), DbError> {
        let next_retry = if !success {
            // Exponential backoff: 30s → 5m → 30m → 2h → 8h (max 5 attempts)
            Some(OffsetDateTime::now_utc() + time::Duration::seconds(30))
        } else {
            None
        };

        let status = if success { "success" } else { "retrying" };

        sqlx::query(
            "UPDATE webhook_deliveries \
             SET status = $2, http_status = $3, response_body = $4, \
                 attempt_count = attempt_count + 1, \
                 next_retry_at = $5, \
                 delivered_at = CASE WHEN $6 THEN NOW() ELSE NULL END, \
                 updated_at = NOW() \
             WHERE id = $1",
        )
        .bind(id)
        .bind(status)
        .bind(http_status)
        .bind(response_body)
        .bind(next_retry)
        .bind(success)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }

    pub async fn mark_failed(pool: &PgPool, id: Uuid) -> Result<(), DbError> {
        sqlx::query(
            "UPDATE webhook_deliveries \
             SET status = 'failed', next_retry_at = NULL, updated_at = NOW() \
             WHERE id = $1",
        )
        .bind(id)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }

    pub async fn list_for_endpoint(
        pool: &PgPool,
        org_id: &str,
        endpoint_id: &str,
        limit: i64,
    ) -> Result<Vec<WebhookDelivery>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let ep_uuid = parse_uuid(endpoint_id)?;

        let rows: Vec<DeliveryRow> = sqlx::query_as(&format!(
            "SELECT {DEL_COLS} FROM webhook_deliveries \
             WHERE organization_id = $1 AND endpoint_id = $2 \
             ORDER BY created_at DESC LIMIT $3"
        ))
        .bind(org_uuid)
        .bind(ep_uuid)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get(pool: &PgPool, org_id: &str, id: &str) -> Result<WebhookDelivery, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let del_uuid = parse_uuid(id)?;

        let row: Option<DeliveryRow> = sqlx::query_as(&format!(
            "SELECT {DEL_COLS} FROM webhook_deliveries \
             WHERE id = $1 AND organization_id = $2"
        ))
        .bind(del_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        row.map(Into::into).ok_or(DbError::NotFound)
    }

    /// Reset a failed delivery for manual retry.
    pub async fn reset_for_retry(pool: &PgPool, org_id: &str, id: &str) -> Result<Uuid, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let del_uuid = parse_uuid(id)?;

        let result = sqlx::query_scalar::<_, Uuid>(
            "UPDATE webhook_deliveries \
             SET status = 'retrying', next_retry_at = NOW(), updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2 AND status = 'failed' \
             RETURNING id",
        )
        .bind(del_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        result.ok_or(DbError::NotFound)
    }

    /// Fetch deliveries due for retry. Returns (delivery, url, secret).
    pub async fn due_for_retry(
        pool: &PgPool,
    ) -> Result<Vec<(WebhookDelivery, String, String)>, DbError> {
        #[derive(sqlx::FromRow)]
        struct DueRow {
            id: Uuid,
            organization_id: Uuid,
            endpoint_id: Uuid,
            event_type: String,
            payload: serde_json::Value,
            status: String,
            http_status: Option<i32>,
            response_body: Option<String>,
            attempt_count: i32,
            next_retry_at: Option<OffsetDateTime>,
            delivered_at: Option<OffsetDateTime>,
            created_at: OffsetDateTime,
            endpoint_secret: String,
            endpoint_url: String,
        }

        let rows: Vec<DueRow> = sqlx::query_as(
            "SELECT d.id, d.organization_id, d.endpoint_id, d.event_type, d.payload, \
                    d.status, d.http_status, d.response_body, d.attempt_count, \
                    d.next_retry_at, d.delivered_at, d.created_at, \
                    e.secret AS endpoint_secret, e.url AS endpoint_url \
             FROM webhook_deliveries d \
             JOIN webhook_endpoints e ON e.id = d.endpoint_id \
             WHERE d.status IN ('pending','retrying') \
               AND d.next_retry_at <= NOW() \
               AND d.attempt_count < 5 \
             ORDER BY d.next_retry_at \
             LIMIT 100",
        )
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let secret = r.endpoint_secret.clone();
                let url = r.endpoint_url.clone();
                let del = WebhookDelivery {
                    id: r.id.to_string(),
                    organization_id: r.organization_id.to_string(),
                    endpoint_id: r.endpoint_id.to_string(),
                    event_type: r.event_type,
                    payload: r.payload,
                    status: r.status,
                    http_status: r.http_status,
                    response_body: r.response_body,
                    attempt_count: r.attempt_count,
                    next_retry_at: r.next_retry_at,
                    delivered_at: r.delivered_at,
                    created_at: r.created_at,
                };
                (del, url, secret)
            })
            .collect())
    }
}
