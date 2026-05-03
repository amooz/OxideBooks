use oxidebooks_core::models::{Attachment, CreateAttachment};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct AttachmentRow {
    id: Uuid,
    organization_id: Uuid,
    entity_type: String,
    entity_id: Uuid,
    file_name: String,
    file_size: i64,
    content_type: String,
    storage_url: String,
    uploaded_by: Option<Uuid>,
    created_at: OffsetDateTime,
}

fn from_row(r: AttachmentRow) -> Attachment {
    Attachment {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        entity_type: r.entity_type,
        entity_id: r.entity_id.to_string(),
        file_name: r.file_name,
        file_size: r.file_size,
        content_type: r.content_type,
        storage_url: r.storage_url,
        uploaded_by: r.uploaded_by.map(|u| u.to_string()),
        created_at: r.created_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

pub struct AttachmentRepo;

impl AttachmentRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Vec<Attachment>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let entity_uuid = parse_uuid(entity_id)?;
        let rows: Vec<AttachmentRow> = sqlx::query_as(
            "SELECT id, organization_id, entity_type, entity_id, file_name, file_size, \
             content_type, storage_url, uploaded_by, created_at \
             FROM attachments \
             WHERE organization_id = $1 AND entity_type = $2 AND entity_id = $3 \
             ORDER BY created_at ASC",
        )
        .bind(org_uuid)
        .bind(entity_type)
        .bind(entity_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        entity_type: &str,
        entity_id: &str,
        uploaded_by: Option<&str>,
        input: CreateAttachment,
    ) -> Result<Attachment, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let entity_uuid = parse_uuid(entity_id)?;
        let uploader_uuid = uploaded_by.map(parse_uuid).transpose()?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO attachments \
             (organization_id, entity_type, entity_id, file_name, file_size, content_type, storage_url, uploaded_by) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING id",
        )
        .bind(org_uuid)
        .bind(entity_type)
        .bind(entity_uuid)
        .bind(&input.file_name)
        .bind(input.file_size)
        .bind(&input.content_type)
        .bind(&input.storage_url)
        .bind(uploader_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: AttachmentRow = sqlx::query_as(
            "SELECT id, organization_id, entity_type, entity_id, file_name, file_size, \
             content_type, storage_url, uploaded_by, created_at \
             FROM attachments WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(from_row(row))
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<Attachment, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: AttachmentRow = sqlx::query_as(
            "DELETE FROM attachments WHERE id = $1 AND organization_id = $2 \
             RETURNING id, organization_id, entity_type, entity_id, file_name, file_size, \
             content_type, storage_url, uploaded_by, created_at",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(from_row(row))
    }
}
