use oxidebooks_core::models::PlaidItem;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct ItemRow {
    id: Uuid,
    organization_id: Uuid,
    bank_account_id: Uuid,
    item_id: String,
    institution_id: Option<String>,
    institution_name: Option<String>,
    is_active: bool,
    last_synced_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<ItemRow> for PlaidItem {
    fn from(r: ItemRow) -> Self {
        PlaidItem {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            bank_account_id: r.bank_account_id.to_string(),
            item_id: r.item_id,
            institution_id: r.institution_id,
            institution_name: r.institution_name,
            is_active: r.is_active,
            last_synced_at: r.last_synced_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

const COLS: &str = "id, organization_id, bank_account_id, item_id, institution_id, \
    institution_name, is_active, last_synced_at, created_at, updated_at";

pub struct PlaidRepo;

impl PlaidRepo {
    pub async fn create_item(
        pool: &PgPool,
        org_id: &str,
        bank_account_id: &str,
        item_id: &str,
        access_token: &str,
        institution_id: Option<&str>,
        institution_name: Option<&str>,
    ) -> Result<PlaidItem, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let acct_uuid = parse_uuid(bank_account_id)?;

        let row: ItemRow = sqlx::query_as(&format!(
            "INSERT INTO plaid_items \
             (organization_id, bank_account_id, item_id, access_token, \
              institution_id, institution_name) \
             VALUES ($1,$2,$3,$4,$5,$6) \
             RETURNING {COLS}"
        ))
        .bind(org_uuid)
        .bind(acct_uuid)
        .bind(item_id)
        .bind(access_token)
        .bind(institution_id)
        .bind(institution_name)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.into())
    }

    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<PlaidItem>, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let rows: Vec<ItemRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM plaid_items \
             WHERE organization_id = $1 ORDER BY created_at DESC"
        ))
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get(pool: &PgPool, org_id: &str, id: &str) -> Result<PlaidItem, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let item_uuid = parse_uuid(id)?;

        let row: Option<ItemRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM plaid_items \
             WHERE id = $1 AND organization_id = $2"
        ))
        .bind(item_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        row.map(Into::into).ok_or(DbError::NotFound)
    }

    pub async fn list_active(pool: &PgPool, org_id: &str) -> Result<Vec<PlaidItemFull>, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let rows = sqlx::query_as::<_, PlaidItemFull>(
            "SELECT id, organization_id, bank_account_id, item_id, access_token, \
              institution_id, institution_name, cursor, is_active, last_synced_at, \
              created_at, updated_at \
             FROM plaid_items \
             WHERE organization_id = $1 AND is_active = TRUE",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows)
    }

    pub async fn update_cursor(pool: &PgPool, id: Uuid, cursor: &str) -> Result<(), DbError> {
        sqlx::query(
            "UPDATE plaid_items SET cursor = $2, last_synced_at = NOW(), updated_at = NOW() \
             WHERE id = $1",
        )
        .bind(id)
        .bind(cursor)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }

    pub async fn disconnect(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let item_uuid = parse_uuid(id)?;

        let result = sqlx::query(
            "UPDATE plaid_items SET is_active = FALSE, updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2",
        )
        .bind(item_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    /// Insert a feed transaction from Plaid, deduplicating by plaid_txn_id.
    /// Returns true if inserted, false if it was a duplicate.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_feed_txn(
        pool: &PgPool,
        org_id: Uuid,
        bank_account_id: Uuid,
        plaid_txn_id: &str,
        txn_date: time::Date,
        description: &str,
        amount: i64,
        txn_type: &str,
    ) -> Result<bool, DbError> {
        // Idempotency check — skip if already imported.
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM plaid_transaction_ids WHERE plaid_txn_id = $1)",
        )
        .bind(plaid_txn_id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        if exists {
            return Ok(false);
        }

        let feed_id: Uuid = sqlx::query_scalar(
            "INSERT INTO bank_feed_transactions \
             (organization_id, bank_account_id, txn_date, description, amount, \
              txn_type, source, status) \
             VALUES ($1,$2,$3,$4,$5,$6,'plaid','pending') \
             ON CONFLICT (bank_account_id, txn_date, amount, description) DO NOTHING \
             RETURNING id",
        )
        .bind(org_id)
        .bind(bank_account_id)
        .bind(txn_date)
        .bind(description)
        .bind(amount)
        .bind(txn_type)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .unwrap_or_else(Uuid::nil); // duplicate by content hash → also a skip

        if feed_id.is_nil() {
            return Ok(false);
        }

        sqlx::query(
            "INSERT INTO plaid_transaction_ids (plaid_txn_id, feed_txn_id) VALUES ($1,$2) \
             ON CONFLICT DO NOTHING",
        )
        .bind(plaid_txn_id)
        .bind(feed_id)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(true)
    }
}

/// Full row including access_token and cursor — only used internally for sync.
#[derive(sqlx::FromRow)]
pub struct PlaidItemFull {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub bank_account_id: Uuid,
    pub item_id: String,
    pub access_token: String,
    pub institution_id: Option<String>,
    pub institution_name: Option<String>,
    pub cursor: Option<String>,
    pub is_active: bool,
    pub last_synced_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
