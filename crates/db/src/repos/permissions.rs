use sqlx::PgPool;
use uuid::Uuid;

use oxidebooks_core::models::Permission;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct PermissionRow {
    id: Uuid,
    name: String,
    description: Option<String>,
}

impl From<PermissionRow> for Permission {
    fn from(r: PermissionRow) -> Self {
        Permission {
            id: r.id.to_string(),
            name: r.name,
            description: r.description,
        }
    }
}

pub struct PermissionRepo;

impl PermissionRepo {
    /// All system permissions, alphabetically sorted.
    pub async fn list(pool: &PgPool) -> Result<Vec<Permission>, DbError> {
        let rows: Vec<PermissionRow> =
            sqlx::query_as("SELECT id, name, description FROM permissions ORDER BY name")
                .fetch_all(pool)
                .await
                .map_err(map_sqlx_err)?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Permission names granted to a specific role.
    pub async fn list_for_role(pool: &PgPool, role_id: &str) -> Result<Vec<String>, DbError> {
        let role_uuid = parse_uuid(role_id)?;

        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT p.name FROM permissions p \
             JOIN role_permissions rp ON rp.permission_id = p.id \
             WHERE rp.role_id = $1 \
             ORDER BY p.name",
        )
        .bind(role_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows.into_iter().map(|(n,)| n).collect())
    }

    /// Permission names granted to a user via their assigned role.
    pub async fn list_for_user(pool: &PgPool, user_id: &str) -> Result<Vec<String>, DbError> {
        let user_uuid = parse_uuid(user_id)?;

        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT p.name FROM permissions p \
             JOIN role_permissions rp ON rp.permission_id = p.id \
             JOIN users u ON u.role_id = rp.role_id \
             WHERE u.id = $1 \
             ORDER BY p.name",
        )
        .bind(user_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows.into_iter().map(|(n,)| n).collect())
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
