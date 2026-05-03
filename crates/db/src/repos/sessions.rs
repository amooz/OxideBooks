use oxidebooks_core::models::Session;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct SessionRow {
    id: Uuid,
    user_id: Uuid,
    organization_id: Uuid,
    user_agent: Option<String>,
    ip: Option<String>,
    created_at: OffsetDateTime,
    expires_at: OffsetDateTime,
}

fn from_row(r: SessionRow) -> Session {
    Session {
        id: r.id.to_string(),
        user_id: r.user_id.to_string(),
        organization_id: r.organization_id.to_string(),
        user_agent: r.user_agent,
        ip: r.ip,
        created_at: r.created_at,
        expires_at: r.expires_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

pub struct SessionRepo;

impl SessionRepo {
    pub async fn create(
        pool: &PgPool,
        user_id: &str,
        org_id: &str,
        jti: &str,
        expires_at: OffsetDateTime,
        user_agent: Option<&str>,
        ip: Option<&str>,
    ) -> Result<(), DbError> {
        let user_uuid = parse_uuid(user_id)?;
        let org_uuid = parse_uuid(org_id)?;
        sqlx::query(
            "INSERT INTO user_sessions (user_id, organization_id, jti, expires_at, user_agent, ip) \
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(user_uuid)
        .bind(org_uuid)
        .bind(jti)
        .bind(expires_at)
        .bind(user_agent)
        .bind(ip)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }

    pub async fn list(pool: &PgPool, user_id: &str) -> Result<Vec<Session>, DbError> {
        let user_uuid = parse_uuid(user_id)?;
        let rows: Vec<SessionRow> = sqlx::query_as(
            "SELECT id, user_id, organization_id, user_agent, ip, created_at, expires_at \
             FROM user_sessions \
             WHERE user_id = $1 AND revoked_at IS NULL AND expires_at > NOW() \
             ORDER BY created_at DESC",
        )
        .bind(user_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn revoke(pool: &PgPool, user_id: &str, session_id: &str) -> Result<(), DbError> {
        let user_uuid = parse_uuid(user_id)?;
        let id_uuid = parse_uuid(session_id)?;
        let n = sqlx::query(
            "UPDATE user_sessions SET revoked_at = NOW() \
             WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL",
        )
        .bind(id_uuid)
        .bind(user_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    pub async fn revoke_all_except(
        pool: &PgPool,
        user_id: &str,
        current_jti: &str,
    ) -> Result<u64, DbError> {
        let user_uuid = parse_uuid(user_id)?;
        let n = sqlx::query(
            "UPDATE user_sessions SET revoked_at = NOW() \
             WHERE user_id = $1 AND jti != $2 AND revoked_at IS NULL",
        )
        .bind(user_uuid)
        .bind(current_jti)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        Ok(n)
    }

    /// Returns true when the session with this jti is actively revoked.
    pub async fn is_revoked(pool: &PgPool, jti: &str) -> Result<bool, DbError> {
        let revoked: bool = sqlx::query_scalar(
            "SELECT EXISTS( \
               SELECT 1 FROM user_sessions \
               WHERE jti = $1 AND revoked_at IS NOT NULL \
             )",
        )
        .bind(jti)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(revoked)
    }
}
