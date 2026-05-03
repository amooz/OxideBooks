use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use oxidebooks_core::models::Role;

use crate::error::{map_sqlx_err, DbError};

use super::permissions::PermissionRepo;

// Fixed UUIDs for system roles — seeded in migration 0002_rbac.sql.
pub const ROLE_OWNER_ID: &str = "00000000-0000-0000-0000-000000000001";
pub const ROLE_ADMIN_ID: &str = "00000000-0000-0000-0000-000000000002";
pub const ROLE_ACCOUNTANT_ID: &str = "00000000-0000-0000-0000-000000000003";
pub const ROLE_VIEWER_ID: &str = "00000000-0000-0000-0000-000000000004";

/// Maps a system role name to its well-known UUID string.
pub fn system_role_id(role_name: &str) -> Option<&'static str> {
    match role_name {
        "owner" => Some(ROLE_OWNER_ID),
        "admin" => Some(ROLE_ADMIN_ID),
        "accountant" => Some(ROLE_ACCOUNTANT_ID),
        "viewer" => Some(ROLE_VIEWER_ID),
        _ => None,
    }
}

#[derive(sqlx::FromRow)]
struct RoleRow {
    id: Uuid,
    org_id: Option<Uuid>,
    name: String,
    is_system: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

pub struct RoleRepo;

impl RoleRepo {
    /// List all roles visible to an org: system roles (org_id IS NULL) + org-custom roles.
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<Role>, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let rows: Vec<RoleRow> = sqlx::query_as(
            "SELECT id, org_id, name, is_system, created_at, updated_at FROM roles \
             WHERE org_id IS NULL OR org_id = $1 \
             ORDER BY is_system DESC, name",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut roles = Vec::with_capacity(rows.len());
        for row in rows {
            let role_id_str = row.id.to_string();
            let permissions = PermissionRepo::list_for_role(pool, &role_id_str).await?;
            roles.push(Role {
                id: role_id_str,
                org_id: row.org_id.map(|u| u.to_string()),
                name: row.name,
                is_system: row.is_system,
                permissions,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }
        Ok(roles)
    }

    /// Get a single role by id (must be visible to the org).
    pub async fn get_by_id(pool: &PgPool, org_id: &str, role_id: &str) -> Result<Role, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let role_uuid = parse_uuid(role_id)?;

        let row: RoleRow = sqlx::query_as(
            "SELECT id, org_id, name, is_system, created_at, updated_at FROM roles \
             WHERE id = $1 AND (org_id IS NULL OR org_id = $2)",
        )
        .bind(role_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        let permissions = PermissionRepo::list_for_role(pool, &row.id.to_string()).await?;
        Ok(Role {
            id: row.id.to_string(),
            org_id: row.org_id.map(|u| u.to_string()),
            name: row.name,
            is_system: row.is_system,
            permissions,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    /// Create a custom role scoped to an org.
    pub async fn create(pool: &PgPool, org_id: &str, name: &str) -> Result<Role, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id = Uuid::new_v4();

        let row: RoleRow = sqlx::query_as(
            "INSERT INTO roles (id, org_id, name, is_system) \
             VALUES ($1, $2, $3, FALSE) \
             RETURNING id, org_id, name, is_system, created_at, updated_at",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(name)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(Role {
            id: row.id.to_string(),
            org_id: row.org_id.map(|u| u.to_string()),
            name: row.name,
            is_system: row.is_system,
            permissions: vec![],
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    /// Assign a named permission to a role. Idempotent.
    pub async fn assign_permission(
        pool: &PgPool,
        org_id: &str,
        role_id: &str,
        permission_name: &str,
    ) -> Result<(), DbError> {
        // Verify the role belongs to the org (or is a system role).
        let _ = Self::get_by_id(pool, org_id, role_id).await?;

        let role_uuid = parse_uuid(role_id)?;

        sqlx::query(
            "INSERT INTO role_permissions (role_id, permission_id) \
             SELECT $1, id FROM permissions WHERE name = $2 \
             ON CONFLICT DO NOTHING",
        )
        .bind(role_uuid)
        .bind(permission_name)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }

    /// Remove a named permission from a role.
    pub async fn remove_permission(
        pool: &PgPool,
        org_id: &str,
        role_id: &str,
        permission_name: &str,
    ) -> Result<(), DbError> {
        let _ = Self::get_by_id(pool, org_id, role_id).await?;

        let role_uuid = parse_uuid(role_id)?;

        sqlx::query(
            "DELETE FROM role_permissions \
             WHERE role_id = $1 \
               AND permission_id = (SELECT id FROM permissions WHERE name = $2)",
        )
        .bind(role_uuid)
        .bind(permission_name)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
