use oxidebooks_core::models::{ContactGroup, CreateContactGroup, UpdateContactGroup};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

const COLS: &str = "cg.id, cg.organization_id, cg.name, cg.description, \
     COUNT(cgm.contact_id) AS member_count, \
     cg.created_at, cg.updated_at";

#[derive(sqlx::FromRow)]
struct GroupRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    description: Option<String>,
    member_count: i64,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<GroupRow> for ContactGroup {
    fn from(r: GroupRow) -> Self {
        ContactGroup {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            name: r.name,
            description: r.description,
            member_count: r.member_count,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

pub struct ContactGroupRepo;

impl ContactGroupRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<ContactGroup>, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let rows: Vec<GroupRow> = sqlx::query_as(&format!(
            "SELECT {COLS} \
             FROM contact_groups cg \
             LEFT JOIN contact_group_members cgm ON cgm.group_id = cg.id \
             WHERE cg.organization_id = $1 \
             GROUP BY cg.id, cg.organization_id, cg.name, cg.description, cg.created_at, cg.updated_at \
             ORDER BY cg.name"
        ))
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows.into_iter().map(ContactGroup::from).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<ContactGroup, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let row: GroupRow = sqlx::query_as(&format!(
            "SELECT {COLS} \
             FROM contact_groups cg \
             LEFT JOIN contact_group_members cgm ON cgm.group_id = cg.id \
             WHERE cg.organization_id = $1 AND cg.id = $2 \
             GROUP BY cg.id, cg.organization_id, cg.name, cg.description, cg.created_at, cg.updated_at"
        ))
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        Ok(ContactGroup::from(row))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateContactGroup,
    ) -> Result<ContactGroup, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO contact_groups (id, organization_id, name, description) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(&input.name)
        .bind(&input.description)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateContactGroup,
    ) -> Result<ContactGroup, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        sqlx::query(
            "UPDATE contact_groups SET \
             name        = COALESCE($3, name), \
             description = COALESCE($4, description), \
             updated_at  = NOW() \
             WHERE id = $1 AND organization_id = $2",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .bind(&input.name)
        .bind(&input.description)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        sqlx::query("DELETE FROM contact_groups WHERE id = $1 AND organization_id = $2")
            .bind(id_uuid)
            .bind(org_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;

        Ok(())
    }

    pub async fn add_member(
        pool: &PgPool,
        org_id: &str,
        group_id: &str,
        contact_id: &str,
    ) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let group_uuid = parse_uuid(group_id)?;
        let contact_uuid = parse_uuid(contact_id)?;

        // Verify group belongs to org.
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM contact_groups WHERE id = $1 AND organization_id = $2")
                .bind(group_uuid)
                .bind(org_uuid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;

        if exists.is_none() {
            return Err(DbError::NotFound);
        }

        sqlx::query(
            "INSERT INTO contact_group_members (group_id, contact_id) \
             VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(group_uuid)
        .bind(contact_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }

    pub async fn remove_member(
        pool: &PgPool,
        org_id: &str,
        group_id: &str,
        contact_id: &str,
    ) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let group_uuid = parse_uuid(group_id)?;
        let contact_uuid = parse_uuid(contact_id)?;

        // Verify group belongs to org.
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM contact_groups WHERE id = $1 AND organization_id = $2")
                .bind(group_uuid)
                .bind(org_uuid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;

        if exists.is_none() {
            return Err(DbError::NotFound);
        }

        sqlx::query("DELETE FROM contact_group_members WHERE group_id = $1 AND contact_id = $2")
            .bind(group_uuid)
            .bind(contact_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;

        Ok(())
    }

    pub async fn list_members(
        pool: &PgPool,
        org_id: &str,
        group_id: &str,
    ) -> Result<Vec<String>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let group_uuid = parse_uuid(group_id)?;

        // Verify group belongs to org.
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM contact_groups WHERE id = $1 AND organization_id = $2")
                .bind(group_uuid)
                .bind(org_uuid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;

        if exists.is_none() {
            return Err(DbError::NotFound);
        }

        let rows: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT contact_id FROM contact_group_members WHERE group_id = $1 ORDER BY added_at",
        )
        .bind(group_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows.into_iter().map(|(id,)| id.to_string()).collect())
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
