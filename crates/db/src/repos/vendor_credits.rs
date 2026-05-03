use oxidebooks_core::models::{
    ApplyVendorCredit, CreateVendorCredit, VendorCredit, VendorCreditApplication, VendorCreditLine,
};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct CreditRow {
    id: Uuid,
    organization_id: Uuid,
    contact_id: Option<Uuid>,
    credit_date: time::Date,
    reference: Option<String>,
    memo: Option<String>,
    status: String,
    total_amount: i64,
    applied_amount: i64,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct CreditLineRow {
    id: Uuid,
    credit_id: Uuid,
    account_id: Option<Uuid>,
    description: Option<String>,
    quantity: i64,
    unit_price: i64,
    tax_rate: i64,
    sort_order: i32,
}

fn line_from_row(r: CreditLineRow) -> VendorCreditLine {
    let line_total =
        r.quantity * r.unit_price / 100 + r.quantity * r.unit_price / 100 * r.tax_rate / 10_000;
    VendorCreditLine {
        id: r.id.to_string(),
        credit_id: r.credit_id.to_string(),
        account_id: r.account_id.map(|u| u.to_string()),
        description: r.description,
        quantity: r.quantity,
        unit_price: r.unit_price,
        tax_rate: r.tax_rate,
        sort_order: r.sort_order,
        line_total,
    }
}

async fn fetch_lines(pool: &PgPool, credit_id: Uuid) -> Result<Vec<VendorCreditLine>, DbError> {
    let rows = sqlx::query_as::<_, CreditLineRow>(
        "SELECT id, credit_id, account_id, description, quantity, unit_price, tax_rate, sort_order
         FROM vendor_credit_lines WHERE credit_id = $1
         ORDER BY sort_order, id",
    )
    .bind(credit_id)
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_err)?;
    Ok(rows.into_iter().map(line_from_row).collect())
}

fn credit_from_parts(r: CreditRow, lines: Vec<VendorCreditLine>) -> VendorCredit {
    let remaining = r.total_amount - r.applied_amount;
    VendorCredit {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        contact_id: r.contact_id.map(|u| u.to_string()),
        credit_date: r.credit_date,
        reference: r.reference,
        memo: r.memo,
        status: r.status,
        total_amount: r.total_amount,
        applied_amount: r.applied_amount,
        remaining_amount: remaining,
        lines,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

const COLS: &str = "id, organization_id, contact_id, credit_date, reference, memo, status,
     total_amount, applied_amount, created_at, updated_at";

pub struct VendorCreditRepo;

impl VendorCreditRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        contact_id: Option<&str>,
    ) -> Result<Vec<VendorCredit>, DbError> {
        let org = parse_uuid(org_id)?;
        let contact = contact_id.map(parse_uuid).transpose()?;
        let rows = sqlx::query_as::<_, CreditRow>(&format!(
            "SELECT {COLS} FROM vendor_credits
             WHERE organization_id = $1
               AND ($2::UUID IS NULL OR contact_id = $2)
             ORDER BY credit_date DESC, created_at DESC"
        ))
        .bind(org)
        .bind(contact)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let lines = fetch_lines(pool, r.id).await?;
            out.push(credit_from_parts(r, lines));
        }
        Ok(out)
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<VendorCredit, DbError> {
        let org = parse_uuid(org_id)?;
        let cid = parse_uuid(id)?;
        let row = sqlx::query_as::<_, CreditRow>(&format!(
            "SELECT {COLS} FROM vendor_credits WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org)
        .bind(cid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        let lines = fetch_lines(pool, row.id).await?;
        Ok(credit_from_parts(row, lines))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateVendorCredit,
    ) -> Result<VendorCredit, DbError> {
        let org = parse_uuid(org_id)?;
        let contact = input.contact_id.as_deref().map(parse_uuid).transpose()?;
        let id = Uuid::new_v4();

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        sqlx::query(
            "INSERT INTO vendor_credits
                (id, organization_id, contact_id, credit_date, reference, memo)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id)
        .bind(org)
        .bind(contact)
        .bind(input.credit_date)
        .bind(&input.reference)
        .bind(&input.memo)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        let mut total: i64 = 0;
        for (i, line) in input.lines.iter().enumerate() {
            let acct = line.account_id.as_deref().map(parse_uuid).transpose()?;
            let lt = line.quantity * line.unit_price / 100
                + line.quantity * line.unit_price / 100 * line.tax_rate / 10_000;
            total += lt;
            sqlx::query(
                "INSERT INTO vendor_credit_lines
                    (credit_id, account_id, description, quantity, unit_price, tax_rate, sort_order)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(id)
            .bind(acct)
            .bind(&line.description)
            .bind(line.quantity)
            .bind(line.unit_price)
            .bind(line.tax_rate)
            .bind(line.sort_order.max(i as i32))
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        sqlx::query("UPDATE vendor_credits SET total_amount = $2 WHERE id = $1")
            .bind(id)
            .bind(total)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn void(pool: &PgPool, org_id: &str, id: &str) -> Result<VendorCredit, DbError> {
        let org = parse_uuid(org_id)?;
        let cid = parse_uuid(id)?;
        let rows = sqlx::query(
            "UPDATE vendor_credits SET status = 'voided', updated_at = now()
             WHERE organization_id = $1 AND id = $2 AND status IN ('open','partially_applied')",
        )
        .bind(org)
        .bind(cid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if rows == 0 {
            return Err(DbError::Conflict(
                "credit cannot be voided in its current state".into(),
            ));
        }
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn apply(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: ApplyVendorCredit,
    ) -> Result<VendorCreditApplication, DbError> {
        let org = parse_uuid(org_id)?;
        let cid = parse_uuid(id)?;
        let bill_uuid = parse_uuid(&input.bill_id)?;

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        let credit = sqlx::query_as::<_, CreditRow>(&format!(
            "SELECT {COLS} FROM vendor_credits
             WHERE organization_id = $1 AND id = $2 AND status != 'voided'
             FOR UPDATE"
        ))
        .bind(org)
        .bind(cid)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        let remaining = credit.total_amount - credit.applied_amount;
        if input.amount <= 0 || input.amount > remaining {
            return Err(DbError::Conflict(format!(
                "amount must be between 1 and {remaining} (remaining)"
            )));
        }

        // Verify the bill belongs to the org
        let bill_exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM vendor_bills WHERE organization_id = $1 AND id = $2")
                .bind(org)
                .bind(bill_uuid)
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_sqlx_err)?;
        if bill_exists.is_none() {
            return Err(DbError::NotFound);
        }

        let app_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO vendor_credit_applications
                (id, organization_id, credit_id, bill_id, amount)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (credit_id, bill_id) DO UPDATE SET amount = EXCLUDED.amount",
        )
        .bind(app_id)
        .bind(org)
        .bind(cid)
        .bind(bill_uuid)
        .bind(input.amount)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        let new_applied = credit.applied_amount + input.amount;
        let new_status = if new_applied >= credit.total_amount {
            "fully_applied"
        } else {
            "partially_applied"
        };

        sqlx::query(
            "UPDATE vendor_credits
             SET applied_amount = $3, status = $4, updated_at = now()
             WHERE id = $1 AND organization_id = $2",
        )
        .bind(cid)
        .bind(org)
        .bind(new_applied)
        .bind(new_status)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;

        let app: (Uuid, Uuid, Uuid, Uuid, i64, OffsetDateTime) = sqlx::query_as(
            "SELECT id, organization_id, credit_id, bill_id, amount, applied_at
             FROM vendor_credit_applications WHERE credit_id = $1 AND bill_id = $2",
        )
        .bind(cid)
        .bind(bill_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(VendorCreditApplication {
            id: app.0.to_string(),
            organization_id: app.1.to_string(),
            credit_id: app.2.to_string(),
            bill_id: app.3.to_string(),
            amount: app.4,
            applied_at: app.5,
        })
    }

    pub async fn list_applications(
        pool: &PgPool,
        org_id: &str,
        credit_id: &str,
    ) -> Result<Vec<VendorCreditApplication>, DbError> {
        let org = parse_uuid(org_id)?;
        let cid = parse_uuid(credit_id)?;
        let rows: Vec<(Uuid, Uuid, Uuid, Uuid, i64, OffsetDateTime)> = sqlx::query_as(
            "SELECT vca.id, vca.organization_id, vca.credit_id, vca.bill_id, vca.amount, vca.applied_at
             FROM vendor_credit_applications vca
             WHERE vca.organization_id = $1 AND vca.credit_id = $2
             ORDER BY vca.applied_at DESC",
        )
        .bind(org)
        .bind(cid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows
            .into_iter()
            .map(|r| VendorCreditApplication {
                id: r.0.to_string(),
                organization_id: r.1.to_string(),
                credit_id: r.2.to_string(),
                bill_id: r.3.to_string(),
                amount: r.4,
                applied_at: r.5,
            })
            .collect())
    }
}
