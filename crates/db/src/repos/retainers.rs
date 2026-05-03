use oxidebooks_core::models::{CreateRetainer, DepositRetainer, Retainer, RetainerTransaction};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct RetainerRow {
    id: Uuid,
    organization_id: Uuid,
    contact_id: Uuid,
    currency: String,
    balance_cents: i64,
    created_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct TxnRow {
    id: Uuid,
    retainer_id: Uuid,
    invoice_id: Option<Uuid>,
    amount: i64,
    txn_type: String,
    created_at: OffsetDateTime,
}

fn retainer_from_row(r: RetainerRow) -> Retainer {
    Retainer {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        contact_id: r.contact_id.to_string(),
        currency: r.currency,
        balance_cents: r.balance_cents,
        created_at: r.created_at,
    }
}

fn txn_from_row(r: TxnRow) -> RetainerTransaction {
    RetainerTransaction {
        id: r.id.to_string(),
        retainer_id: r.retainer_id.to_string(),
        invoice_id: r.invoice_id.map(|u| u.to_string()),
        amount: r.amount,
        txn_type: r.txn_type,
        created_at: r.created_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

pub struct RetainerRepo;

impl RetainerRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<Retainer>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<RetainerRow> = sqlx::query_as(
            "SELECT id, organization_id, contact_id, currency, balance_cents, created_at \
             FROM retainers WHERE organization_id = $1 ORDER BY created_at DESC",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(retainer_from_row).collect())
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateRetainer,
    ) -> Result<Retainer, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = parse_uuid(&input.contact_id)?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO retainers (organization_id, contact_id, currency) \
             VALUES ($1,$2,$3) RETURNING id",
        )
        .bind(org_uuid)
        .bind(contact_uuid)
        .bind(&input.currency)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: RetainerRow = sqlx::query_as(
            "SELECT id, organization_id, contact_id, currency, balance_cents, created_at \
             FROM retainers WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(retainer_from_row(row))
    }

    pub async fn deposit(
        pool: &PgPool,
        org_id: &str,
        retainer_id: &str,
        input: DepositRetainer,
    ) -> Result<Retainer, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let ret_uuid = parse_uuid(retainer_id)?;

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        let n = sqlx::query(
            "UPDATE retainers SET balance_cents = balance_cents + $1 \
             WHERE id = $2 AND organization_id = $3",
        )
        .bind(input.amount)
        .bind(ret_uuid)
        .bind(org_uuid)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::NotFound);
        }

        sqlx::query(
            "INSERT INTO retainer_transactions (retainer_id, amount, txn_type) VALUES ($1,$2,'deposit')",
        )
        .bind(ret_uuid)
        .bind(input.amount)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;

        let row: RetainerRow = sqlx::query_as(
            "SELECT id, organization_id, contact_id, currency, balance_cents, created_at \
             FROM retainers WHERE id = $1",
        )
        .bind(ret_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(retainer_from_row(row))
    }

    pub async fn apply(
        pool: &PgPool,
        org_id: &str,
        retainer_id: &str,
        invoice_id: &str,
        amount: i64,
    ) -> Result<Retainer, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let ret_uuid = parse_uuid(retainer_id)?;
        let inv_uuid = parse_uuid(invoice_id)?;

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        let balance: i64 = sqlx::query_scalar(
            "SELECT balance_cents FROM retainers WHERE id = $1 AND organization_id = $2 FOR UPDATE",
        )
        .bind(ret_uuid)
        .bind(org_uuid)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        if balance < amount {
            return Err(DbError::Conflict("insufficient retainer balance".into()));
        }

        sqlx::query("UPDATE retainers SET balance_cents = balance_cents - $1 WHERE id = $2")
            .bind(amount)
            .bind(ret_uuid)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;

        sqlx::query(
            "INSERT INTO retainer_transactions (retainer_id, invoice_id, amount, txn_type) \
             VALUES ($1,$2,$3,'applied')",
        )
        .bind(ret_uuid)
        .bind(inv_uuid)
        .bind(amount)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;

        let row: RetainerRow = sqlx::query_as(
            "SELECT id, organization_id, contact_id, currency, balance_cents, created_at \
             FROM retainers WHERE id = $1",
        )
        .bind(ret_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(retainer_from_row(row))
    }

    pub async fn list_transactions(
        pool: &PgPool,
        org_id: &str,
        retainer_id: &str,
    ) -> Result<Vec<RetainerTransaction>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let ret_uuid = parse_uuid(retainer_id)?;
        let rows: Vec<TxnRow> = sqlx::query_as(
            "SELECT rt.id, rt.retainer_id, rt.invoice_id, rt.amount, rt.txn_type, rt.created_at \
             FROM retainer_transactions rt \
             JOIN retainers r ON r.id = rt.retainer_id \
             WHERE rt.retainer_id = $1 AND r.organization_id = $2 \
             ORDER BY rt.created_at DESC",
        )
        .bind(ret_uuid)
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(txn_from_row).collect())
    }
}
