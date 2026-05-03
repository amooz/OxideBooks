use oxidebooks_core::models::{EmailLog, EmailSettings, UpsertEmailSettings};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct EmailSettingsRow {
    organization_id: Uuid,
    smtp_host: String,
    smtp_port: i32,
    smtp_user: String,
    from_address: String,
    from_name: String,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct EmailLogRow {
    id: Uuid,
    organization_id: Uuid,
    to_address: String,
    subject: String,
    entity_type: Option<String>,
    entity_id: Option<Uuid>,
    status: String,
    error: Option<String>,
    created_at: OffsetDateTime,
}

fn settings_from_row(r: EmailSettingsRow) -> EmailSettings {
    EmailSettings {
        organization_id: r.organization_id.to_string(),
        smtp_host: r.smtp_host,
        smtp_port: r.smtp_port,
        smtp_user: r.smtp_user,
        from_address: r.from_address,
        from_name: r.from_name,
        updated_at: r.updated_at,
    }
}

fn log_from_row(r: EmailLogRow) -> EmailLog {
    EmailLog {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        to_address: r.to_address,
        subject: r.subject,
        entity_type: r.entity_type,
        entity_id: r.entity_id.map(|u| u.to_string()),
        status: r.status,
        error: r.error,
        created_at: r.created_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

pub struct EmailRepo;

impl EmailRepo {
    pub async fn get_settings(pool: &PgPool, org_id: &str) -> Result<EmailSettings, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let row: EmailSettingsRow = sqlx::query_as(
            "SELECT organization_id, smtp_host, smtp_port, smtp_user, from_address, from_name, updated_at \
             FROM email_settings WHERE organization_id = $1",
        )
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(settings_from_row(row))
    }

    pub async fn upsert_settings(
        pool: &PgPool,
        org_id: &str,
        input: UpsertEmailSettings,
    ) -> Result<EmailSettings, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        sqlx::query(
            "INSERT INTO email_settings \
             (organization_id, smtp_host, smtp_port, smtp_user, smtp_password, from_address, from_name) \
             VALUES ($1,$2,$3,$4,$5,$6,$7) \
             ON CONFLICT (organization_id) DO UPDATE SET \
               smtp_host     = EXCLUDED.smtp_host, \
               smtp_port     = EXCLUDED.smtp_port, \
               smtp_user     = EXCLUDED.smtp_user, \
               smtp_password = EXCLUDED.smtp_password, \
               from_address  = EXCLUDED.from_address, \
               from_name     = EXCLUDED.from_name, \
               updated_at    = now()",
        )
        .bind(org_uuid)
        .bind(&input.smtp_host)
        .bind(input.smtp_port)
        .bind(&input.smtp_user)
        .bind(&input.smtp_password)
        .bind(&input.from_address)
        .bind(&input.from_name)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;
        Self::get_settings(pool, org_id).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_log(
        pool: &PgPool,
        org_id: &str,
        to_address: &str,
        subject: &str,
        entity_type: Option<&str>,
        entity_id: Option<&str>,
        status: &str,
        error: Option<&str>,
    ) -> Result<EmailLog, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let entity_uuid = entity_id.map(parse_uuid).transpose()?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO email_log \
             (organization_id, to_address, subject, entity_type, entity_id, status, error) \
             VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id",
        )
        .bind(org_uuid)
        .bind(to_address)
        .bind(subject)
        .bind(entity_type)
        .bind(entity_uuid)
        .bind(status)
        .bind(error)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: EmailLogRow = sqlx::query_as(
            "SELECT id, organization_id, to_address, subject, entity_type, entity_id, \
             status, error, created_at FROM email_log WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(log_from_row(row))
    }

    pub async fn list_log(pool: &PgPool, org_id: &str) -> Result<Vec<EmailLog>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<EmailLogRow> = sqlx::query_as(
            "SELECT id, organization_id, to_address, subject, entity_type, entity_id, \
             status, error, created_at \
             FROM email_log WHERE organization_id = $1 \
             ORDER BY created_at DESC LIMIT 200",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(log_from_row).collect())
    }
}
