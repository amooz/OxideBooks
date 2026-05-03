use oxidebooks_core::models::{BankDeposit, BankDepositItem, CreateBankDeposit};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct DepositRow {
    id: Uuid,
    organization_id: Uuid,
    bank_account_id: Uuid,
    deposit_date: Date,
    currency: String,
    total_amount: i64,
    reference: Option<String>,
    memo: Option<String>,
    status: String,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct ItemRow {
    id: Uuid,
    deposit_id: Uuid,
    payment_id: Uuid,
    amount: i64,
}

async fn fetch_items(pool: &PgPool, deposit_id: Uuid) -> Result<Vec<BankDepositItem>, DbError> {
    let rows: Vec<ItemRow> = sqlx::query_as(
        "SELECT id, deposit_id, payment_id, amount
         FROM bank_deposit_items WHERE deposit_id = $1 ORDER BY id",
    )
    .bind(deposit_id)
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_err)?;
    Ok(rows
        .into_iter()
        .map(|r| BankDepositItem {
            id: r.id.to_string(),
            deposit_id: r.deposit_id.to_string(),
            payment_id: r.payment_id.to_string(),
            amount: r.amount,
        })
        .collect())
}

async fn deposit_from_row(pool: &PgPool, r: DepositRow) -> Result<BankDeposit, DbError> {
    let items = fetch_items(pool, r.id).await?;
    Ok(BankDeposit {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        bank_account_id: r.bank_account_id.to_string(),
        deposit_date: r.deposit_date,
        currency: r.currency,
        total_amount: r.total_amount,
        reference: r.reference,
        memo: r.memo,
        status: r.status,
        items,
        created_at: r.created_at,
        updated_at: r.updated_at,
    })
}

const DCOLS: &str = "id, organization_id, bank_account_id, deposit_date, currency,
     total_amount, reference, memo, status, created_at, updated_at";

pub struct BankDepositRepo;

impl BankDepositRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        bank_account_id: Option<&str>,
    ) -> Result<Vec<BankDeposit>, DbError> {
        let org = parse_uuid(org_id)?;
        let ba = bank_account_id.map(parse_uuid).transpose()?;
        let rows: Vec<DepositRow> = sqlx::query_as(&format!(
            "SELECT {DCOLS} FROM bank_deposits
             WHERE organization_id = $1
               AND ($2::UUID IS NULL OR bank_account_id = $2)
             ORDER BY deposit_date DESC, created_at DESC"
        ))
        .bind(org)
        .bind(ba)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(deposit_from_row(pool, r).await?);
        }
        Ok(out)
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<BankDeposit, DbError> {
        let org = parse_uuid(org_id)?;
        let did = parse_uuid(id)?;
        let row: DepositRow = sqlx::query_as(&format!(
            "SELECT {DCOLS} FROM bank_deposits WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org)
        .bind(did)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        deposit_from_row(pool, row).await
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateBankDeposit,
    ) -> Result<BankDeposit, DbError> {
        let org = parse_uuid(org_id)?;
        let ba = parse_uuid(&input.bank_account_id)?;

        if input.items.is_empty() {
            return Err(DbError::Conflict(
                "deposit must have at least one item".into(),
            ));
        }

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        let currency = input.currency.unwrap_or_else(|| "USD".to_string());
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO bank_deposits
                (organization_id, bank_account_id, deposit_date, currency, reference, memo)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id",
        )
        .bind(org)
        .bind(ba)
        .bind(input.deposit_date)
        .bind(&currency)
        .bind(&input.reference)
        .bind(&input.memo)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        let mut total: i64 = 0;
        for item in &input.items {
            let pay_id = parse_uuid(&item.payment_id)?;
            // Verify payment belongs to org
            let exists: Option<(Uuid,)> =
                sqlx::query_as("SELECT id FROM payments WHERE organization_id = $1 AND id = $2")
                    .bind(org)
                    .bind(pay_id)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(map_sqlx_err)?;
            if exists.is_none() {
                return Err(DbError::NotFound);
            }
            sqlx::query(
                "INSERT INTO bank_deposit_items (deposit_id, payment_id, amount)
                 VALUES ($1, $2, $3)",
            )
            .bind(id)
            .bind(pay_id)
            .bind(item.amount)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
            total += item.amount;
        }

        sqlx::query("UPDATE bank_deposits SET total_amount = $2, updated_at = now() WHERE id = $1")
            .bind(id)
            .bind(total)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn clear(pool: &PgPool, org_id: &str, id: &str) -> Result<BankDeposit, DbError> {
        let org = parse_uuid(org_id)?;
        let did = parse_uuid(id)?;
        let n = sqlx::query(
            "UPDATE bank_deposits SET status = 'cleared', updated_at = now()
             WHERE organization_id = $1 AND id = $2 AND status = 'open'",
        )
        .bind(org)
        .bind(did)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::Conflict(
                "deposit not found or already cleared".into(),
            ));
        }
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org = parse_uuid(org_id)?;
        let did = parse_uuid(id)?;
        let n = sqlx::query(
            "DELETE FROM bank_deposits
             WHERE organization_id = $1 AND id = $2 AND status = 'open'",
        )
        .bind(org)
        .bind(did)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::Conflict(
                "deposit not found or already cleared".into(),
            ));
        }
        Ok(())
    }
}
