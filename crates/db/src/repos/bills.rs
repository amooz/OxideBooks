use oxidebooks_core::models::{
    BillLine, BillPayment, CreateBillPayment, CreateVendorBill, UpdateVendorBill, VendorBill,
};
use rust_decimal::Decimal;
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct BillRow {
    id: Uuid,
    contact_id: Option<Uuid>,
    bill_date: Date,
    due_date: Option<Date>,
    reference: Option<String>,
    description: String,
    status: String,
    doc_number: Option<String>,
    currency_code: String,
    exchange_rate: Decimal,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct BillLineRow {
    id: Uuid,
    bill_id: Uuid,
    account_id: Option<Uuid>,
    description: Option<String>,
    quantity: i32,
    unit_price: i64,
    tax_rate: i64,
}

#[derive(sqlx::FromRow)]
struct BillPaymentRow {
    id: Uuid,
    organization_id: Uuid,
    bill_id: Uuid,
    payment_date: Date,
    amount: i64,
    method: String,
    reference: Option<String>,
    created_at: OffsetDateTime,
}

impl From<BillPaymentRow> for BillPayment {
    fn from(r: BillPaymentRow) -> Self {
        BillPayment {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            bill_id: r.bill_id.to_string(),
            payment_date: r.payment_date,
            amount: r.amount,
            method: r.method,
            reference: r.reference,
            created_at: r.created_at,
        }
    }
}

pub struct BillRepo;

impl BillRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<VendorBill>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<BillRow> = sqlx::query_as(
            "SELECT id, contact_id, bill_date, due_date, reference, \
             description, status, doc_number, currency_code, exchange_rate, \
             created_at, updated_at \
             FROM vendor_bills WHERE organization_id = $1 \
             ORDER BY bill_date DESC, created_at DESC",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut bills = Vec::with_capacity(rows.len());
        for r in rows {
            bills.push(Self::assemble(pool, org_uuid, r).await?);
        }
        Ok(bills)
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<VendorBill, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: BillRow = sqlx::query_as(
            "SELECT id, contact_id, bill_date, due_date, reference, \
             description, status, doc_number, currency_code, exchange_rate, \
             created_at, updated_at \
             FROM vendor_bills WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Self::assemble(pool, org_uuid, row).await
    }

    pub async fn list_for_contact(
        pool: &PgPool,
        org_id: &str,
        contact_id: &str,
    ) -> Result<Vec<VendorBill>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = parse_uuid(contact_id)?;
        let rows: Vec<BillRow> = sqlx::query_as(
            "SELECT id, contact_id, bill_date, due_date, reference, \
             description, status, doc_number, currency_code, exchange_rate, \
             created_at, updated_at \
             FROM vendor_bills WHERE organization_id = $1 AND contact_id = $2 \
             ORDER BY bill_date DESC, created_at DESC",
        )
        .bind(org_uuid)
        .bind(contact_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut bills = Vec::with_capacity(rows.len());
        for r in rows {
            bills.push(Self::assemble(pool, org_uuid, r).await?);
        }
        Ok(bills)
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateVendorBill,
    ) -> Result<VendorBill, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = input.contact_id.as_deref().map(parse_uuid).transpose()?;
        let po_uuid = input
            .purchase_order_id
            .as_deref()
            .map(parse_uuid)
            .transpose()?;
        let id = Uuid::new_v4();

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        sqlx::query(
            "INSERT INTO vendor_bills \
             (id, organization_id, contact_id, bill_date, due_date, reference, description, \
              currency_code, exchange_rate, purchase_order_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(contact_uuid)
        .bind(input.bill_date)
        .bind(input.due_date)
        .bind(&input.reference)
        .bind(&input.description)
        .bind(&input.currency_code)
        .bind(input.exchange_rate)
        .bind(po_uuid)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        for line in &input.lines {
            let line_id = Uuid::new_v4();
            let acct_uuid = line.account_id.as_deref().map(parse_uuid).transpose()?;
            sqlx::query(
                "INSERT INTO bill_lines \
                 (id, bill_id, account_id, description, quantity, unit_price, tax_rate) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(line_id)
            .bind(id)
            .bind(acct_uuid)
            .bind(&line.description)
            .bind(line.quantity)
            .bind(line.unit_price)
            .bind(line.tax_rate)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateVendorBill,
    ) -> Result<VendorBill, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let rows_affected = sqlx::query(
            "UPDATE vendor_bills SET \
             contact_id  = COALESCE($3, contact_id), \
             bill_date   = COALESCE($4, bill_date), \
             due_date    = COALESCE($5, due_date), \
             reference   = COALESCE($6, reference), \
             description = COALESCE($7, description), \
             updated_at  = NOW() \
             WHERE organization_id = $1 AND id = $2 AND status = 'draft'",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .bind(input.contact_id.as_deref().map(parse_uuid).transpose()?)
        .bind(input.bill_date)
        .bind(input.due_date)
        .bind(&input.reference)
        .bind(&input.description)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows_affected == 0 {
            return Err(DbError::Conflict(
                "bill not found or cannot be updated (only draft bills can be modified)".into(),
            ));
        }
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn approve(pool: &PgPool, org_id: &str, id: &str) -> Result<VendorBill, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows = sqlx::query(
            "UPDATE vendor_bills SET status = 'approved', updated_at = NOW() \
             WHERE organization_id = $1 AND id = $2 AND status = 'draft'",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if rows == 0 {
            return Err(DbError::Conflict(
                "bill not found or not in draft state".into(),
            ));
        }
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn void(pool: &PgPool, org_id: &str, id: &str) -> Result<VendorBill, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows = sqlx::query(
            "UPDATE vendor_bills SET status = 'voided', updated_at = NOW() \
             WHERE organization_id = $1 AND id = $2 AND status IN ('draft','approved')",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if rows == 0 {
            let bill = Self::get_by_id(pool, org_id, id).await?;
            return Err(DbError::Conflict(format!(
                "cannot void bill with status '{}'",
                bill.status
            )));
        }
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn record_payment(
        pool: &PgPool,
        org_id: &str,
        bill_id: &str,
        input: CreateBillPayment,
    ) -> Result<BillPayment, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let bill_uuid = parse_uuid(bill_id)?;

        // Verify bill exists and is payable.
        let bill = Self::get_by_id(pool, org_id, bill_id).await?;
        if bill.status == "voided" || bill.status == "paid" {
            return Err(DbError::Conflict(format!(
                "cannot record payment against a {} bill",
                bill.status
            )));
        }
        let outstanding = bill.total - bill.amount_paid;
        if input.amount <= 0 || input.amount > outstanding {
            return Err(DbError::Conflict(format!(
                "payment amount must be between 1 and {} (outstanding)",
                outstanding
            )));
        }

        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO bill_payments \
             (id, organization_id, bill_id, payment_date, amount, method, reference) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(bill_uuid)
        .bind(input.payment_date)
        .bind(input.amount)
        .bind(&input.method)
        .bind(&input.reference)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::sync_bill_status(pool, org_uuid, bill_uuid).await?;

        let row: BillPaymentRow = sqlx::query_as(
            "SELECT id, organization_id, bill_id, payment_date, amount, method, reference, created_at \
             FROM bill_payments WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.into())
    }

    pub async fn list_payments(
        pool: &PgPool,
        org_id: &str,
        bill_id: &str,
    ) -> Result<Vec<BillPayment>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let bill_uuid = parse_uuid(bill_id)?;
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM vendor_bills WHERE organization_id = $1 AND id = $2")
                .bind(org_uuid)
                .bind(bill_uuid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;
        if exists.is_none() {
            return Err(DbError::NotFound);
        }
        let rows: Vec<BillPaymentRow> = sqlx::query_as(
            "SELECT id, organization_id, bill_id, payment_date, amount, method, reference, created_at \
             FROM bill_payments WHERE bill_id = $1 ORDER BY payment_date ASC, created_at ASC",
        )
        .bind(bill_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(BillPayment::from).collect())
    }

    async fn sync_bill_status(
        pool: &PgPool,
        org_uuid: Uuid,
        bill_uuid: Uuid,
    ) -> Result<(), DbError> {
        let bill_total: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(quantity * unit_price + quantity * unit_price * tax_rate / 1000000), 0)::BIGINT \
             FROM bill_lines WHERE bill_id = $1",
        )
        .bind(bill_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let paid: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(amount), 0)::BIGINT FROM bill_payments WHERE bill_id = $1",
        )
        .bind(bill_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let new_status = if paid.0 >= bill_total.0 {
            "paid"
        } else if paid.0 > 0 {
            "partial"
        } else {
            return Ok(());
        };

        sqlx::query(
            "UPDATE vendor_bills SET status = $1, updated_at = NOW() \
             WHERE organization_id = $2 AND id = $3 AND status NOT IN ('voided','paid')",
        )
        .bind(new_status)
        .bind(org_uuid)
        .bind(bill_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }

    async fn assemble(pool: &PgPool, org_uuid: Uuid, r: BillRow) -> Result<VendorBill, DbError> {
        let line_rows: Vec<BillLineRow> = sqlx::query_as(
            "SELECT id, bill_id, account_id, description, quantity, unit_price, tax_rate \
             FROM bill_lines WHERE bill_id = $1",
        )
        .bind(r.id)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let lines: Vec<BillLine> = line_rows
            .into_iter()
            .map(|l| BillLine {
                id: l.id.to_string(),
                bill_id: l.bill_id.to_string(),
                account_id: l.account_id.map(|u| u.to_string()),
                description: l.description,
                quantity: l.quantity,
                unit_price: l.unit_price,
                tax_rate: l.tax_rate,
            })
            .collect();

        let total: i64 = lines.iter().map(|l| l.line_total()).sum();

        let paid: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(amount), 0)::BIGINT FROM bill_payments WHERE bill_id = $1",
        )
        .bind(r.id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(VendorBill {
            id: r.id.to_string(),
            organization_id: org_uuid.to_string(),
            contact_id: r.contact_id.map(|u| u.to_string()),
            bill_date: r.bill_date,
            due_date: r.due_date,
            reference: r.reference,
            description: r.description,
            status: r.status,
            doc_number: r.doc_number,
            currency_code: r.currency_code,
            exchange_rate: r.exchange_rate,
            lines,
            total,
            amount_paid: paid.0,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
