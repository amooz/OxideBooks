use oxidebooks_core::models::{CreateNote, Note};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct NoteRow {
    id: Uuid,
    organization_id: Uuid,
    user_id: Option<Uuid>,
    entity_type: String,
    entity_id: Uuid,
    body: String,
    is_system: bool,
    created_at: OffsetDateTime,
}

fn from_row(r: NoteRow) -> Note {
    Note {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        user_id: r.user_id.map(|u| u.to_string()),
        entity_type: r.entity_type,
        entity_id: r.entity_id.to_string(),
        body: r.body,
        is_system: r.is_system,
        created_at: r.created_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

pub struct NoteRepo;

impl NoteRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Vec<Note>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let entity_uuid = parse_uuid(entity_id)?;
        let rows: Vec<NoteRow> = sqlx::query_as(
            "SELECT id, organization_id, user_id, entity_type, entity_id, body, is_system, created_at \
             FROM notes \
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
        user_id: Option<&str>,
        entity_type: &str,
        entity_id: &str,
        input: CreateNote,
        is_system: bool,
    ) -> Result<Note, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let entity_uuid = parse_uuid(entity_id)?;
        let user_uuid = user_id.map(parse_uuid).transpose()?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO notes (organization_id, user_id, entity_type, entity_id, body, is_system) \
             VALUES ($1,$2,$3,$4,$5,$6) RETURNING id",
        )
        .bind(org_uuid)
        .bind(user_uuid)
        .bind(entity_type)
        .bind(entity_uuid)
        .bind(&input.body)
        .bind(is_system)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: NoteRow = sqlx::query_as(
            "SELECT id, organization_id, user_id, entity_type, entity_id, body, is_system, created_at \
             FROM notes WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(from_row(row))
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n = sqlx::query("DELETE FROM notes WHERE id = $1 AND organization_id = $2")
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
}
