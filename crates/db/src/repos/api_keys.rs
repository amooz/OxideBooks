use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use oxidebooks_core::models::{ApiKey, CreateApiKey, CreatedApiKey};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct ApiKeyRow {
    id: Uuid,
    organization_id: Uuid,
    user_id: Uuid,
    name: String,
    key_prefix: String,
    scopes: Vec<String>,
    is_active: bool,
    last_used_at: Option<OffsetDateTime>,
    expires_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
}

fn from_row(r: ApiKeyRow) -> ApiKey {
    ApiKey {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        user_id: r.user_id.to_string(),
        name: r.name,
        key_prefix: r.key_prefix,
        scopes: r.scopes,
        is_active: r.is_active,
        last_used_at: r.last_used_at,
        expires_at: r.expires_at,
        created_at: r.created_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

fn generate_key() -> (String, String, String) {
    let mut bytes = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
    let plaintext = format!("obok_{}", URL_SAFE_NO_PAD.encode(bytes));
    let prefix = plaintext[..12].to_string();
    let salt = SaltString::generate(&mut OsRng);
    let hashed = Argon2::default()
        .hash_password(plaintext.as_bytes(), &salt)
        .expect("hashing failed")
        .to_string();
    (plaintext, prefix, hashed)
}

pub struct ApiKeyRepo;

impl ApiKeyRepo {
    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        user_id: &str,
        input: CreateApiKey,
    ) -> Result<CreatedApiKey, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let user_uuid = parse_uuid(user_id)?;
        let (plaintext, prefix, hashed) = generate_key();

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO api_keys \
             (organization_id, user_id, name, key_prefix, hashed_key, scopes, expires_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id",
        )
        .bind(org_uuid)
        .bind(user_uuid)
        .bind(&input.name)
        .bind(&prefix)
        .bind(&hashed)
        .bind(&input.scopes)
        .bind(input.expires_at)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: ApiKeyRow = sqlx::query_as(
            "SELECT id, organization_id, user_id, name, key_prefix, scopes, is_active, \
             last_used_at, expires_at, created_at FROM api_keys WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(CreatedApiKey {
            key: from_row(row),
            plaintext,
        })
    }

    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<ApiKey>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<ApiKeyRow> = sqlx::query_as(
            "SELECT id, organization_id, user_id, name, key_prefix, scopes, is_active, \
             last_used_at, expires_at, created_at \
             FROM api_keys WHERE organization_id = $1 ORDER BY created_at DESC",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn revoke(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n = sqlx::query(
            "UPDATE api_keys SET is_active = false WHERE id = $1 AND organization_id = $2",
        )
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

    /// Verify a plaintext API key against stored hashes. Returns the org_id and user_id on success.
    pub async fn authenticate(
        pool: &PgPool,
        plaintext: &str,
    ) -> Result<Option<(String, String)>, DbError> {
        if plaintext.len() < 12 {
            return Ok(None);
        }
        let prefix = &plaintext[..12];

        #[derive(sqlx::FromRow)]
        struct Row {
            id: Uuid,
            organization_id: Uuid,
            user_id: Uuid,
            hashed_key: String,
            expires_at: Option<OffsetDateTime>,
        }

        let candidates: Vec<Row> = sqlx::query_as(
            "SELECT id, organization_id, user_id, hashed_key, expires_at \
             FROM api_keys WHERE key_prefix = $1 AND is_active = true",
        )
        .bind(prefix)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let now = OffsetDateTime::now_utc();
        for row in candidates {
            if let Some(exp) = row.expires_at {
                if exp < now {
                    continue;
                }
            }
            if let Ok(parsed) = PasswordHash::new(&row.hashed_key) {
                if Argon2::default()
                    .verify_password(plaintext.as_bytes(), &parsed)
                    .is_ok()
                {
                    // Update last_used_at fire-and-forget
                    let pool2 = pool.clone();
                    let id = row.id;
                    tokio::spawn(async move {
                        let _ =
                            sqlx::query("UPDATE api_keys SET last_used_at = now() WHERE id = $1")
                                .bind(id)
                                .execute(&pool2)
                                .await;
                    });
                    return Ok(Some((
                        row.organization_id.to_string(),
                        row.user_id.to_string(),
                    )));
                }
            }
        }
        Ok(None)
    }
}
