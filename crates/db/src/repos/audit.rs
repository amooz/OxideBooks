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
    pub severity: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub changes: Option<serde_json::Value>,
    #[serde(with = "time::serde::rfc3339")]
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
    severity: String,
    ip_address: Option<String>,
    user_agent: Option<String>,
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
            severity: r.severity,
            ip_address: r.ip_address,
            user_agent: r.user_agent,
            changes: r.changes,
            created_at: r.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceSummaryRow {
    pub user_id: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub severity: String,
    pub event_count: i64,
}

#[derive(sqlx::FromRow)]
struct SummaryRow {
    user_id: Option<Uuid>,
    action: String,
    resource_type: String,
    severity: String,
    event_count: i64,
}

impl From<SummaryRow> for ComplianceSummaryRow {
    fn from(r: SummaryRow) -> Self {
        ComplianceSummaryRow {
            user_id: r.user_id.map(|u| u.to_string()),
            action: r.action,
            resource_type: r.resource_type,
            severity: r.severity,
            event_count: r.event_count,
        }
    }
}

const COLS: &str = "id, organization_id, user_id, action, resource_type, resource_id, \
    severity, ip_address::TEXT, user_agent, changes, created_at";

pub struct AuditRepo;

impl AuditRepo {
    /// Record an audit event. Errors are intentionally ignored at the call site
    /// so that audit failures do not affect the primary operation.
    #[allow(clippy::too_many_arguments)]
    pub async fn record(
        pool: &PgPool,
        org_id: &str,
        user_id: Option<&str>,
        action: &str,
        resource_type: &str,
        resource_id: &str,
        changes: Option<serde_json::Value>,
    ) -> Result<(), DbError> {
        Self::record_full(
            pool,
            org_id,
            user_id,
            action,
            resource_type,
            resource_id,
            changes,
            "info",
            None,
            None,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn record_full(
        pool: &PgPool,
        org_id: &str,
        user_id: Option<&str>,
        action: &str,
        resource_type: &str,
        resource_id: &str,
        changes: Option<serde_json::Value>,
        severity: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let user_uuid = user_id.map(parse_uuid).transpose()?;

        sqlx::query(
            "INSERT INTO audit_events \
             (organization_id, user_id, action, resource_type, resource_id, \
              changes, severity, ip_address, user_agent) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8::inet,$9)",
        )
        .bind(org_uuid)
        .bind(user_uuid)
        .bind(action)
        .bind(resource_type)
        .bind(resource_id)
        .bind(changes)
        .bind(severity)
        .bind(ip_address)
        .bind(user_agent)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        resource_type: Option<&str>,
        resource_id: Option<&str>,
        user_id: Option<&str>,
        severity: Option<&str>,
        since: Option<OffsetDateTime>,
        until: Option<OffsetDateTime>,
        page: &PageParams,
    ) -> Result<(Vec<AuditEvent>, Option<String>), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let user_uuid = user_id.map(parse_uuid).transpose()?;
        let limit = page.limit_clamped();
        let cursor = page.decode_cursor();

        let rows: Vec<AuditRow> = if let Some(c) = cursor {
            let cursor_ts = time::OffsetDateTime::parse(
                &c.created_at,
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(|_| DbError::Conflict("invalid cursor".into()))?;
            let cursor_id = parse_uuid(&c.id)?;

            sqlx::query_as(&format!(
                "SELECT {COLS} FROM audit_events \
                 WHERE organization_id = $1 \
                   AND ($2::text IS NULL OR resource_type = $2) \
                   AND ($3::text IS NULL OR resource_id = $3) \
                   AND ($4::uuid IS NULL OR user_id = $4) \
                   AND ($5::text IS NULL OR severity = $5) \
                   AND ($6::timestamptz IS NULL OR created_at >= $6) \
                   AND ($7::timestamptz IS NULL OR created_at <= $7) \
                   AND (created_at, id) < ($8, $9) \
                 ORDER BY created_at DESC, id DESC LIMIT $10"
            ))
            .bind(org_uuid)
            .bind(resource_type)
            .bind(resource_id)
            .bind(user_uuid)
            .bind(severity)
            .bind(since)
            .bind(until)
            .bind(cursor_ts)
            .bind(cursor_id)
            .bind(limit + 1)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(&format!(
                "SELECT {COLS} FROM audit_events \
                 WHERE organization_id = $1 \
                   AND ($2::text IS NULL OR resource_type = $2) \
                   AND ($3::text IS NULL OR resource_id = $3) \
                   AND ($4::uuid IS NULL OR user_id = $4) \
                   AND ($5::text IS NULL OR severity = $5) \
                   AND ($6::timestamptz IS NULL OR created_at >= $6) \
                   AND ($7::timestamptz IS NULL OR created_at <= $7) \
                 ORDER BY created_at DESC, id DESC LIMIT $8"
            ))
            .bind(org_uuid)
            .bind(resource_type)
            .bind(resource_id)
            .bind(user_uuid)
            .bind(severity)
            .bind(since)
            .bind(until)
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

    /// Fetch all events in a date range for export (no pagination cap).
    #[allow(clippy::too_many_arguments)]
    pub async fn list_for_export(
        pool: &PgPool,
        org_id: &str,
        resource_type: Option<&str>,
        user_id: Option<&str>,
        severity: Option<&str>,
        since: Option<OffsetDateTime>,
        until: Option<OffsetDateTime>,
    ) -> Result<Vec<AuditEvent>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let user_uuid = user_id.map(parse_uuid).transpose()?;

        let rows: Vec<AuditRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM audit_events \
             WHERE organization_id = $1 \
               AND ($2::text IS NULL OR resource_type = $2) \
               AND ($3::uuid IS NULL OR user_id = $3) \
               AND ($4::text IS NULL OR severity = $4) \
               AND ($5::timestamptz IS NULL OR created_at >= $5) \
               AND ($6::timestamptz IS NULL OR created_at <= $6) \
             ORDER BY created_at DESC \
             LIMIT 100000"
        ))
        .bind(org_uuid)
        .bind(resource_type)
        .bind(user_uuid)
        .bind(severity)
        .bind(since)
        .bind(until)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows.into_iter().map(AuditEvent::from).collect())
    }

    /// Compliance summary: event counts grouped by user, action, resource_type, severity.
    pub async fn compliance_summary(
        pool: &PgPool,
        org_id: &str,
        since: Option<OffsetDateTime>,
        until: Option<OffsetDateTime>,
    ) -> Result<Vec<ComplianceSummaryRow>, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let rows: Vec<SummaryRow> = sqlx::query_as(
            "SELECT user_id, action, resource_type, severity, COUNT(*) AS event_count \
             FROM audit_events \
             WHERE organization_id = $1 \
               AND ($2::timestamptz IS NULL OR created_at >= $2) \
               AND ($3::timestamptz IS NULL OR created_at <= $3) \
             GROUP BY user_id, action, resource_type, severity \
             ORDER BY event_count DESC",
        )
        .bind(org_uuid)
        .bind(since)
        .bind(until)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Fetch all events for a specific resource (no pagination).
    pub async fn get_for_resource(
        pool: &PgPool,
        org_id: &str,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<Vec<AuditEvent>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<AuditRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM audit_events \
             WHERE organization_id = $1 AND resource_type = $2 AND resource_id = $3 \
             ORDER BY created_at DESC LIMIT 500"
        ))
        .bind(org_uuid)
        .bind(resource_type)
        .bind(resource_id)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(AuditEvent::from).collect())
    }

    /// Delete audit events older than the given number of days. Returns count deleted.
    pub async fn purge_old(
        pool: &PgPool,
        org_id: &str,
        older_than_days: i64,
    ) -> Result<u64, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let n = sqlx::query(
            "DELETE FROM audit_events \
             WHERE organization_id = $1 \
               AND created_at < NOW() - ($2::bigint * INTERVAL '1 day')",
        )
        .bind(org_uuid)
        .bind(older_than_days)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        Ok(n)
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
