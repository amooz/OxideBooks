use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use oxidebooks_core::models::{
    CreateDirectDepositBatch, CreateDirectDepositEntry, DirectDepositBatch, DirectDepositEntry,
    MarkBatchSent,
};

use crate::error::{map_sqlx_err, DbError};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

const BATCH_COLS: &str = "id, organization_id, payroll_run_id, bank_account_id, batch_date,
    status, total_amount, entry_count, reference, sent_at, created_at, updated_at";

const ENTRY_COLS: &str = "id, batch_id, employee_id, employee_bank_id,
    amount, routing_number, account_number, account_type";

#[derive(sqlx::FromRow)]
struct BatchRow {
    id: Uuid,
    organization_id: Uuid,
    payroll_run_id: Option<Uuid>,
    bank_account_id: Option<Uuid>,
    batch_date: Date,
    status: String,
    total_amount: i64,
    entry_count: i32,
    reference: Option<String>,
    sent_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct EntryRow {
    id: Uuid,
    batch_id: Uuid,
    employee_id: Uuid,
    employee_bank_id: Option<Uuid>,
    amount: i64,
    routing_number: Option<String>,
    account_number: Option<String>,
    account_type: String,
}

impl From<EntryRow> for DirectDepositEntry {
    fn from(r: EntryRow) -> Self {
        DirectDepositEntry {
            id: r.id.to_string(),
            batch_id: r.batch_id.to_string(),
            employee_id: r.employee_id.to_string(),
            employee_bank_id: r.employee_bank_id.map(|u| u.to_string()),
            amount: r.amount,
            routing_number: r.routing_number,
            account_number: r.account_number,
            account_type: r.account_type,
        }
    }
}

fn to_batch(r: BatchRow, entries: Vec<DirectDepositEntry>) -> DirectDepositBatch {
    DirectDepositBatch {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        payroll_run_id: r.payroll_run_id.map(|u| u.to_string()),
        bank_account_id: r.bank_account_id.map(|u| u.to_string()),
        batch_date: r.batch_date,
        status: r.status,
        total_amount: r.total_amount,
        entry_count: r.entry_count,
        reference: r.reference,
        sent_at: r.sent_at,
        entries,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

async fn fetch_entries(pool: &PgPool, batch_id: Uuid) -> Result<Vec<DirectDepositEntry>, DbError> {
    let rows = sqlx::query_as::<_, EntryRow>(&format!(
        "SELECT {ENTRY_COLS} FROM direct_deposit_entries WHERE batch_id = $1 ORDER BY id"
    ))
    .bind(batch_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub struct DirectDepositRepo;

impl DirectDepositRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<DirectDepositBatch>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows = sqlx::query_as::<_, BatchRow>(&format!(
            "SELECT {BATCH_COLS} FROM direct_deposit_batches
             WHERE organization_id = $1 ORDER BY batch_date DESC, id"
        ))
        .bind(org_uuid)
        .fetch_all(pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let entries = fetch_entries(pool, row.id).await?;
            out.push(to_batch(row, entries));
        }
        Ok(out)
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<DirectDepositBatch, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let batch_uuid = parse_uuid(id)?;
        let row = sqlx::query_as::<_, BatchRow>(&format!(
            "SELECT {BATCH_COLS} FROM direct_deposit_batches
             WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(batch_uuid)
        .fetch_optional(pool)
        .await?
        .ok_or(DbError::NotFound)?;

        let entries = fetch_entries(pool, row.id).await?;
        Ok(to_batch(row, entries))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateDirectDepositBatch,
    ) -> Result<DirectDepositBatch, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let payroll_run_uuid = input
            .payroll_run_id
            .as_deref()
            .map(parse_uuid)
            .transpose()?;
        let bank_uuid = input
            .bank_account_id
            .as_deref()
            .map(parse_uuid)
            .transpose()?;

        let total: i64 = input.entries.iter().map(|e| e.amount).sum();
        let count = input.entries.len() as i32;

        let row = sqlx::query_as::<_, BatchRow>(&format!(
            "INSERT INTO direct_deposit_batches
                (organization_id, payroll_run_id, bank_account_id, batch_date,
                 total_amount, entry_count, reference)
             VALUES ($1,$2,$3,$4,$5,$6,$7)
             RETURNING {BATCH_COLS}"
        ))
        .bind(org_uuid)
        .bind(payroll_run_uuid)
        .bind(bank_uuid)
        .bind(input.batch_date)
        .bind(total)
        .bind(count)
        .bind(&input.reference)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let entries = insert_entries(pool, row.id, &input.entries).await?;
        Ok(to_batch(row, entries))
    }

    pub async fn mark_sent(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: MarkBatchSent,
    ) -> Result<DirectDepositBatch, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let batch_uuid = parse_uuid(id)?;

        let row = sqlx::query_as::<_, BatchRow>(&format!(
            "UPDATE direct_deposit_batches
             SET status = 'sent', sent_at = now(),
                 reference = COALESCE($3, reference), updated_at = now()
             WHERE organization_id = $1 AND id = $2 AND status = 'pending'
             RETURNING {BATCH_COLS}"
        ))
        .bind(org_uuid)
        .bind(batch_uuid)
        .bind(input.reference)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        let entries = fetch_entries(pool, row.id).await?;
        Ok(to_batch(row, entries))
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let batch_uuid = parse_uuid(id)?;
        let result = sqlx::query(
            "DELETE FROM direct_deposit_batches
             WHERE organization_id = $1 AND id = $2 AND status = 'pending'",
        )
        .bind(org_uuid)
        .bind(batch_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }
}

async fn insert_entries(
    pool: &PgPool,
    batch_id: Uuid,
    entries: &[CreateDirectDepositEntry],
) -> Result<Vec<DirectDepositEntry>, DbError> {
    let mut out = Vec::with_capacity(entries.len());
    for e in entries {
        let emp_uuid = parse_uuid(&e.employee_id)?;
        let bank_uuid = e.employee_bank_id.as_deref().map(parse_uuid).transpose()?;

        let row = sqlx::query_as::<_, EntryRow>(&format!(
            "INSERT INTO direct_deposit_entries
                (batch_id, employee_id, employee_bank_id, amount,
                 routing_number, account_number, account_type)
             VALUES ($1,$2,$3,$4,$5,$6,$7)
             RETURNING {ENTRY_COLS}"
        ))
        .bind(batch_id)
        .bind(emp_uuid)
        .bind(bank_uuid)
        .bind(e.amount)
        .bind(&e.routing_number)
        .bind(&e.account_number)
        .bind(&e.account_type)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        out.push(row.into());
    }
    Ok(out)
}
