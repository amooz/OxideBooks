use oxidebooks_core::models::{CreateSalesCommission, PayCommission, SalesCommission};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct CommissionRow {
    id: Uuid,
    organization_id: Uuid,
    invoice_id: Uuid,
    salesperson_id: Uuid,
    rate_bps: i32,
    amount: i64,
    status: String,
    payment_date: Option<Date>,
    payment_ref: Option<String>,
    notes: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn from_row(r: CommissionRow) -> SalesCommission {
    SalesCommission {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        invoice_id: r.invoice_id.to_string(),
        salesperson_id: r.salesperson_id.to_string(),
        rate_bps: r.rate_bps,
        amount: r.amount,
        status: r.status,
        payment_date: r.payment_date,
        payment_ref: r.payment_ref,
        notes: r.notes,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

const COLS: &str = "id, organization_id, invoice_id, salesperson_id, rate_bps, amount, \
                    status, payment_date, payment_ref, notes, created_at, updated_at";

pub struct CommissionRepo;

impl CommissionRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<SalesCommission>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<CommissionRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM sales_commissions \
             WHERE organization_id = $1 ORDER BY created_at DESC"
        ))
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn list_for_invoice(
        pool: &PgPool,
        org_id: &str,
        invoice_id: &str,
    ) -> Result<Vec<SalesCommission>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let inv_uuid = parse_uuid(invoice_id)?;
        let rows: Vec<CommissionRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM sales_commissions \
             WHERE organization_id = $1 AND invoice_id = $2 ORDER BY created_at ASC"
        ))
        .bind(org_uuid)
        .bind(inv_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<SalesCommission, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: CommissionRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM sales_commissions \
             WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(from_row(row))
    }

    /// Creates a commission for an invoice, computing the amount from the invoice total.
    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateSalesCommission,
    ) -> Result<SalesCommission, DbError> {
        if input.rate_bps < 0 || input.rate_bps > 100_000 {
            return Err(DbError::Conflict("rate_bps must be 0–100000".into()));
        }
        let org_uuid = parse_uuid(org_id)?;
        let inv_uuid = parse_uuid(&input.invoice_id)?;
        let person_uuid = parse_uuid(&input.salesperson_id)?;

        // Compute amount from the invoice total (minor units × rate_bps / 10000).
        let invoice_total: Option<i64> = sqlx::query_scalar(
            "SELECT total_amount FROM invoices WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(inv_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        let invoice_total = invoice_total.ok_or(DbError::NotFound)?;
        let amount = (invoice_total as i128 * input.rate_bps as i128 / 10_000) as i64;

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO sales_commissions \
             (organization_id, invoice_id, salesperson_id, rate_bps, amount, notes) \
             VALUES ($1,$2,$3,$4,$5,$6) RETURNING id",
        )
        .bind(org_uuid)
        .bind(inv_uuid)
        .bind(person_uuid)
        .bind(input.rate_bps)
        .bind(amount)
        .bind(&input.notes)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    /// Approves a pending commission.
    pub async fn approve(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<SalesCommission, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows_affected = sqlx::query(
            "UPDATE sales_commissions SET status = 'approved', updated_at = NOW() \
             WHERE organization_id = $1 AND id = $2 AND status = 'pending'",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if rows_affected == 0 {
            return Err(DbError::Conflict(
                "commission must be in pending status to approve".into(),
            ));
        }
        Self::get_by_id(pool, org_id, id).await
    }

    /// Marks a commission as paid.
    pub async fn pay(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: PayCommission,
    ) -> Result<SalesCommission, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows_affected = sqlx::query(
            "UPDATE sales_commissions SET status = 'paid', payment_date = $1, \
             payment_ref = $2, updated_at = NOW() \
             WHERE organization_id = $3 AND id = $4 AND status = 'approved'",
        )
        .bind(input.payment_date)
        .bind(&input.payment_ref)
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if rows_affected == 0 {
            return Err(DbError::Conflict(
                "commission must be in approved status to mark paid".into(),
            ));
        }
        Self::get_by_id(pool, org_id, id).await
    }

    /// Voids a commission (any status except already voided).
    pub async fn void(pool: &PgPool, org_id: &str, id: &str) -> Result<SalesCommission, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows_affected = sqlx::query(
            "UPDATE sales_commissions SET status = 'voided', updated_at = NOW() \
             WHERE organization_id = $1 AND id = $2 AND status != 'voided'",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if rows_affected == 0 {
            return Err(DbError::NotFound);
        }
        Self::get_by_id(pool, org_id, id).await
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
