use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use oxidebooks_core::models::ClientPortalToken;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct TokenRow {
    id: Uuid,
    token: String,
    contact_id: Uuid,
    organization_id: Uuid,
    expires_at: OffsetDateTime,
    created_at: OffsetDateTime,
}

fn from_row(r: TokenRow) -> ClientPortalToken {
    ClientPortalToken {
        id: r.id.to_string(),
        token: r.token,
        contact_id: r.contact_id.to_string(),
        organization_id: r.organization_id.to_string(),
        expires_at: r.expires_at,
        created_at: r.created_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

fn generate_token() -> String {
    let mut bytes = [0u8; 24];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
    format!("portal_{}", URL_SAFE_NO_PAD.encode(bytes))
}

pub struct ClientPortalRepo;

impl ClientPortalRepo {
    pub async fn create_token(
        pool: &PgPool,
        org_id: &str,
        contact_id: &str,
        expires_at: OffsetDateTime,
    ) -> Result<ClientPortalToken, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = parse_uuid(contact_id)?;
        let token = generate_token();

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO client_portal_tokens (token, contact_id, organization_id, expires_at) \
             VALUES ($1,$2,$3,$4) RETURNING id",
        )
        .bind(&token)
        .bind(contact_uuid)
        .bind(org_uuid)
        .bind(expires_at)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: TokenRow = sqlx::query_as(
            "SELECT id, token, contact_id, organization_id, expires_at, created_at \
             FROM client_portal_tokens WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(from_row(row))
    }

    /// Returns the token if it exists and has not expired.
    pub async fn get_by_token(pool: &PgPool, token: &str) -> Result<ClientPortalToken, DbError> {
        let row: TokenRow = sqlx::query_as(
            "SELECT id, token, contact_id, organization_id, expires_at, created_at \
             FROM client_portal_tokens \
             WHERE token = $1 AND expires_at > NOW()",
        )
        .bind(token)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(from_row(row))
    }
}
