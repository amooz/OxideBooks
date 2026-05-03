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
