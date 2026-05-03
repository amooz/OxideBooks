use oxidebooks_core::pagination::{encode_cursor, PageParams};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use super::roles::system_role_id;
use crate::error::{map_sqlx_err, DbError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub organization_id: String,
    pub email: String,
    pub name: String,
    /// Role name resolved via JOIN with the roles table.
    pub role: String,
    pub is_active: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

pub struct UserWithHash {
    pub user: User,
    pub password_hash: String,
}

pub struct CreateUser {
    pub organization_id: String,
    pub email: String,
    pub password_hash: String,
    pub name: String,
    /// Role name (e.g. "owner", "admin"). Resolved to a role_id internally.
    pub role: String,
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    organization_id: Uuid,
    email: String,
    name: String,
    role_name: String,
    is_active: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct UserRowWithHash {
    id: Uuid,
    organization_id: Uuid,
    email: String,
    password_hash: String,
    name: String,
    role_name: String,
    is_active: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

pub struct UpdateUser {
    pub role: Option<String>,
    pub is_active: Option<bool>,
}

pub struct UserRepo;

impl UserRepo {
    pub async fn create(pool: &PgPool, input: CreateUser) -> Result<User, DbError> {
        let id = Uuid::new_v4();
        let org_uuid = parse_uuid(&input.organization_id)?;

        // Look up role_id: prefer the system role by name, falling back to
        // an org-custom role with that name.
        let role_id_str = system_role_id(&input.role)
            .map(String::from)
            .ok_or_else(|| DbError::Conflict(format!("unknown role: {}", input.role)))?;
        let role_uuid = parse_uuid(&role_id_str)?;

        sqlx::query(
            "INSERT INTO users (id, organization_id, email, password_hash, name, role_id) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(&input.email)
        .bind(&input.password_hash)
        .bind(&input.name)
        .bind(role_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, &id.to_string()).await
    }

    pub async fn get_by_id(pool: &PgPool, id: &str) -> Result<User, DbError> {
        let id_uuid = parse_uuid(id)?;

        let row: UserRow = sqlx::query_as(
            "SELECT u.id, u.organization_id, u.email, u.name, r.name AS role_name, \
                    u.is_active, u.created_at, u.updated_at \
             FROM users u \
             JOIN roles r ON r.id = u.role_id \
             WHERE u.id = $1",
        )
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        Ok(row.into())
    }

    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        page: &PageParams,
    ) -> Result<(Vec<User>, Option<String>), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let limit = page.limit_clamped();
        let cursor = page.decode_cursor();

        let rows: Vec<UserRow> = if let Some(c) = cursor {
            let cursor_ts = time::OffsetDateTime::parse(
                &c.created_at,
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(|_| DbError::Conflict("invalid cursor".into()))?;
            let cursor_id = parse_uuid(&c.id)?;
            sqlx::query_as(
                "SELECT u.id, u.organization_id, u.email, u.name, r.name AS role_name, \
                        u.is_active, u.created_at, u.updated_at \
                 FROM users u \
                 JOIN roles r ON r.id = u.role_id \
                 WHERE u.organization_id = $1 AND (u.created_at, u.id) > ($2, $3) \
                 ORDER BY u.created_at ASC, u.id ASC LIMIT $4",
            )
            .bind(org_uuid)
            .bind(cursor_ts)
            .bind(cursor_id)
            .bind(limit + 1)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(
                "SELECT u.id, u.organization_id, u.email, u.name, r.name AS role_name, \
                        u.is_active, u.created_at, u.updated_at \
                 FROM users u \
                 JOIN roles r ON r.id = u.role_id \
                 WHERE u.organization_id = $1 \
                 ORDER BY u.created_at ASC, u.id ASC LIMIT $2",
            )
            .bind(org_uuid)
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
        Ok((rows.into_iter().map(User::from).collect(), next_cursor))
    }

    pub async fn get_by_id_with_hash(pool: &PgPool, id: &str) -> Result<UserWithHash, DbError> {
        let id_uuid = parse_uuid(id)?;

        let row: UserRowWithHash = sqlx::query_as(
            "SELECT u.id, u.organization_id, u.email, u.password_hash, u.name, \
                    r.name AS role_name, u.is_active, u.created_at, u.updated_at \
             FROM users u \
             JOIN roles r ON r.id = u.role_id \
             WHERE u.id = $1",
        )
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        Ok(UserWithHash {
            password_hash: row.password_hash,
            user: User {
                id: row.id.to_string(),
                organization_id: row.organization_id.to_string(),
                email: row.email,
                name: row.name,
                role: row.role_name,
                is_active: row.is_active,
                created_at: row.created_at,
                updated_at: row.updated_at,
            },
        })
    }

    pub async fn update_password(
        pool: &PgPool,
        user_id: &str,
        new_hash: &str,
    ) -> Result<(), DbError> {
        let id_uuid = parse_uuid(user_id)?;

        let rows_affected =
            sqlx::query("UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2")
                .bind(new_hash)
                .bind(id_uuid)
                .execute(pool)
                .await
                .map_err(map_sqlx_err)?
                .rows_affected();

        if rows_affected == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        user_id: &str,
        input: UpdateUser,
    ) -> Result<User, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(user_id)?;

        if let Some(ref role_name) = input.role {
            let role_id_str = super::roles::system_role_id(role_name)
                .map(String::from)
                .ok_or_else(|| DbError::Conflict(format!("unknown role: {role_name}")))?;
            let role_uuid = parse_uuid(&role_id_str)?;
            sqlx::query(
                "UPDATE users SET role_id = $1, updated_at = NOW() \
                 WHERE id = $2 AND organization_id = $3",
            )
            .bind(role_uuid)
            .bind(id_uuid)
            .bind(org_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;
        }

        if let Some(is_active) = input.is_active {
            sqlx::query(
                "UPDATE users SET is_active = $1, updated_at = NOW() \
                 WHERE id = $2 AND organization_id = $3",
            )
            .bind(is_active)
            .bind(id_uuid)
            .bind(org_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;
        }

        Self::get_by_id(pool, &id_uuid.to_string()).await
    }

    pub async fn deactivate(pool: &PgPool, org_id: &str, user_id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(user_id)?;

        let rows_affected = sqlx::query(
            "UPDATE users SET is_active = FALSE, updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows_affected == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    pub async fn get_by_email(
        pool: &PgPool,
        org_id: &str,
        email: &str,
    ) -> Result<UserWithHash, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let row: UserRowWithHash = sqlx::query_as(
            "SELECT u.id, u.organization_id, u.email, u.password_hash, u.name, \
                    r.name AS role_name, u.is_active, u.created_at, u.updated_at \
             FROM users u \
             JOIN roles r ON r.id = u.role_id \
             WHERE u.organization_id = $1 AND u.email = $2 AND u.is_active = TRUE",
        )
        .bind(org_uuid)
        .bind(email)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        let user = User {
            id: row.id.to_string(),
            organization_id: row.organization_id.to_string(),
            email: row.email,
            name: row.name,
            role: row.role_name,
            is_active: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
        };

        Ok(UserWithHash {
            password_hash: row.password_hash,
            user,
        })
    }
}

impl From<UserRow> for User {
    fn from(r: UserRow) -> Self {
        User {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            email: r.email,
            name: r.name,
            role: r.role_name,
            is_active: r.is_active,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
