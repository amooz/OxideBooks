use oxidebooks_core::pagination::{encode_cursor, PageParams};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub organization_id: String,
    pub user_id: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub changes: Option<serde_json::Value>,
    pub created_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct AuditRow {
    id: Uuid,
    organization_id: Uuid,
    user_id: Option<Uuid>,
    action: String,
    resource_type: String,
    resource_id: String,
    changes: Option<serde_json::Value>,
    created_at: OffsetDateTime,
}

impl From<AuditRow> for AuditEvent {
    fn from(r: AuditRow) -> Self {
        AuditEvent {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            user_id: r.user_id.map(|u| u.to_string()),
            action: r.action,
            resource_type: r.resource_type,
            resource_id: r.resource_id,
            changes: r.changes,
            created_at: r.created_at,
        }
    }
}

pub struct AuditRepo;

impl AuditRepo {
    /// Record an audit event. Errors are intentionally ignored at the call site
    /// so that audit failures do not affect the primary operation.
    pub async fn record(
        pool: &PgPool,
        org_id: &str,
        user_id: Option<&str>,
        action: &str,
        resource_type: &str,
        resource_id: &str,
        changes: Option<serde_json::Value>,
    ) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let user_uuid = user_id.map(parse_uuid).transpose()?;

        sqlx::query(
            "INSERT INTO audit_events \
             (organization_id, user_id, action, resource_type, resource_id, changes) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(org_uuid)
        .bind(user_uuid)
        .bind(action)
        .bind(resource_type)
        .bind(resource_id)
        .bind(changes)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }

    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        resource_type: Option<&str>,
        resource_id: Option<&str>,
        page: &PageParams,
    ) -> Result<(Vec<AuditEvent>, Option<String>), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let limit = page.limit_clamped();
        let cursor = page.decode_cursor();

        // Build query with optional filters using a common approach.
        let rows: Vec<AuditRow> = if let Some(c) = cursor {
            let cursor_ts = time::OffsetDateTime::parse(
                &c.created_at,
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(|_| DbError::Conflict("invalid cursor".into()))?;
            let cursor_id = parse_uuid(&c.id)?;

            sqlx::query_as(
                "SELECT id, organization_id, user_id, action, resource_type, resource_id, \
                 changes, created_at FROM audit_events \
                 WHERE organization_id = $1 \
                   AND ($2::text IS NULL OR resource_type = $2) \
                   AND ($3::text IS NULL OR resource_id = $3) \
                   AND (created_at, id) < ($4, $5) \
                 ORDER BY created_at DESC, id DESC LIMIT $6",
            )
            .bind(org_uuid)
            .bind(resource_type)
            .bind(resource_id)
            .bind(cursor_ts)
            .bind(cursor_id)
            .bind(limit + 1)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(
                "SELECT id, organization_id, user_id, action, resource_type, resource_id, \
                 changes, created_at FROM audit_events \
                 WHERE organization_id = $1 \
                   AND ($2::text IS NULL OR resource_type = $2) \
                   AND ($3::text IS NULL OR resource_id = $3) \
                 ORDER BY created_at DESC, id DESC LIMIT $4",
            )
            .bind(org_uuid)
            .bind(resource_type)
            .bind(resource_id)
            .bind(limit + 1)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        };

        let has_next = rows.len() as i64 > limit;
        let mut rows = rows;
        if has_next {
            rows.pop();
        }
        let next_cursor = if has_next {
            rows.last()
                .map(|r| encode_cursor(r.created_at, &r.id.to_string()))
        } else {
            None
        };

        Ok((
            rows.into_iter().map(AuditEvent::from).collect(),
            next_cursor,
        ))
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
