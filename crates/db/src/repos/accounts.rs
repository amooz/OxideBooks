use oxidebooks_core::models::{Account, AccountType, CreateAccount, UpdateAccount};
use sqlx::PgPool;
use std::str::FromStr;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct AccountRow {
    id: Uuid,
    organization_id: Uuid,
    code: String,
    name: String,
    account_type: String,
    parent_id: Option<Uuid>,
    description: Option<String>,
    is_active: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl AccountRow {
    fn into_account(self) -> Result<Account, DbError> {
        Ok(Account {
            id: self.id.to_string(),
            organization_id: self.organization_id.to_string(),
            code: self.code,
            name: self.name,
            account_type: AccountType::from_str(&self.account_type)?,
            parent_id: self.parent_id.map(|u| u.to_string()),
            description: self.description,
            is_active: self.is_active,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

pub struct AccountRepo;

impl AccountRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<Account>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<AccountRow> = sqlx::query_as(
            "SELECT id, organization_id, code, name, account_type, parent_id, description, \
             is_active, created_at, updated_at \
             FROM accounts WHERE organization_id = $1 ORDER BY code",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        rows.into_iter().map(|r| r.into_account()).collect()
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<Account, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let row: AccountRow = sqlx::query_as(
            "SELECT id, organization_id, code, name, account_type, parent_id, description, \
             is_active, created_at, updated_at \
             FROM accounts WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        row.into_account()
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateAccount,
    ) -> Result<Account, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id = Uuid::new_v4();
        let account_type = input.account_type.to_string();
        let parent_uuid = input
            .parent_id
            .as_deref()
            .map(parse_uuid)
            .transpose()?;

        sqlx::query(
            "INSERT INTO accounts \
             (id, organization_id, code, name, account_type, parent_id, description) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(&input.code)
        .bind(&input.name)
        .bind(&account_type)
        .bind(parent_uuid)
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
        input: UpdateAccount,
    ) -> Result<Account, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        sqlx::query(
            "UPDATE accounts SET \
             code        = COALESCE($3, code), \
             name        = COALESCE($4, name), \
             description = COALESCE($5, description), \
             is_active   = COALESCE($6, is_active), \
             updated_at  = NOW() \
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .bind(&input.code)
        .bind(&input.name)
        .bind(&input.description)
        .bind(input.is_active)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let result = sqlx::query(
            "DELETE FROM accounts WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| {
        DbError::Conflict(format!("invalid UUID: {s}"))
    })
}
