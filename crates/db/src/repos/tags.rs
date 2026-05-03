use oxidebooks_core::models::{CreateTag, Tag, UpdateTag};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct TagRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    color: String,
    created_at: OffsetDateTime,
}

fn from_row(r: TagRow) -> Tag {
    Tag {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        name: r.name,
        color: r.color,
        created_at: r.created_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

pub struct TagRepo;

impl TagRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<Tag>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<TagRow> = sqlx::query_as(
            "SELECT id, organization_id, name, color, created_at \
             FROM tags WHERE organization_id = $1 ORDER BY name",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<Tag, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: TagRow = sqlx::query_as(
            "SELECT id, organization_id, name, color, created_at \
             FROM tags WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(from_row(row))
    }

    pub async fn create(pool: &PgPool, org_id: &str, input: CreateTag) -> Result<Tag, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO tags (organization_id, name, color) VALUES ($1,$2,$3) RETURNING id",
        )
        .bind(org_uuid)
        .bind(&input.name)
        .bind(&input.color)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateTag,
    ) -> Result<Tag, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n = sqlx::query(
            "UPDATE tags SET \
             name  = COALESCE($1, name), \
             color = COALESCE($2, color) \
             WHERE id = $3 AND organization_id = $4",
        )
        .bind(input.name)
        .bind(input.color)
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
        let n = sqlx::query("DELETE FROM tags WHERE id = $1 AND organization_id = $2")
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

    pub async fn list_for_entity(pool: &PgPool, entity_id: &str) -> Result<Vec<Tag>, DbError> {
        let entity_uuid = parse_uuid(entity_id)?;
        let rows: Vec<TagRow> = sqlx::query_as(
            "SELECT t.id, t.organization_id, t.name, t.color, t.created_at \
             FROM tags t \
             JOIN entity_tags et ON et.tag_id = t.id \
             WHERE et.entity_id = $1 \
             ORDER BY t.name",
        )
        .bind(entity_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn add_tag_to_entity(
        pool: &PgPool,
        org_id: &str,
        tag_id: &str,
        entity_id: &str,
        entity_type: &str,
    ) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let tag_uuid = parse_uuid(tag_id)?;
        let entity_uuid = parse_uuid(entity_id)?;

        // Verify tag belongs to org
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM tags WHERE id = $1 AND organization_id = $2)",
        )
        .bind(tag_uuid)
        .bind(org_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        if !exists {
            return Err(DbError::NotFound);
        }

        sqlx::query(
            "INSERT INTO entity_tags (tag_id, entity_id, entity_type) \
             VALUES ($1,$2,$3) ON CONFLICT DO NOTHING",
        )
        .bind(tag_uuid)
        .bind(entity_uuid)
        .bind(entity_type)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }

    pub async fn remove_tag_from_entity(
        pool: &PgPool,
        org_id: &str,
        tag_id: &str,
        entity_id: &str,
    ) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let tag_uuid = parse_uuid(tag_id)?;
        let entity_uuid = parse_uuid(entity_id)?;

        // Verify tag belongs to org
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM tags WHERE id = $1 AND organization_id = $2)",
        )
        .bind(tag_uuid)
        .bind(org_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        if !exists {
            return Err(DbError::NotFound);
        }

        sqlx::query("DELETE FROM entity_tags WHERE tag_id = $1 AND entity_id = $2")
            .bind(tag_uuid)
            .bind(entity_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;
        Ok(())
    }
}
