use oxidebooks_core::models::{CreatePayment, CreateRefund, Payment, Refund};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct PaymentRow {
    id: Uuid,
    organization_id: Uuid,
    invoice_id: Uuid,
    amount: i64,
    payment_date: Date,
    method: String,
    reference: Option<String>,
    notes: Option<String>,
    status: String,
    realized_fx_amount: i64,
    fx_journal_entry_id: Option<Uuid>,
    voided_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
}

impl From<PaymentRow> for Payment {
    fn from(r: PaymentRow) -> Self {
        Payment {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            invoice_id: r.invoice_id.to_string(),
            amount: r.amount,
            payment_date: r.payment_date,
            method: r.method,
            reference: r.reference,
            notes: r.notes,
            status: r.status,
            realized_fx_amount: r.realized_fx_amount,
            fx_journal_entry_id: r.fx_journal_entry_id.map(|u| u.to_string()),
            voided_at: r.voided_at,
            created_at: r.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct RefundRow {
    id: Uuid,
    payment_id: Uuid,
    amount: i64,
    reason: Option<String>,
    refund_date: Date,
    created_at: OffsetDateTime,
}

impl From<RefundRow> for Refund {
    fn from(r: RefundRow) -> Self {
        Refund {
            id: r.id.to_string(),
            payment_id: r.payment_id.to_string(),
            amount: r.amount,
            reason: r.reason,
            refund_date: r.refund_date,
            created_at: r.created_at,
        }
    }
}

const PAYMENT_COLS: &str = "id, organization_id, invoice_id, amount, payment_date, method, \
     reference, notes, status, realized_fx_amount, fx_journal_entry_id, voided_at, created_at";

pub struct PaymentRepo;

impl PaymentRepo {
    /// Record a payment against an invoice and auto-update its status.
    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        invoice_id: &str,
        input: CreatePayment,
    ) -> Result<Payment, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let inv_uuid = parse_uuid(invoice_id)?;
        let id = Uuid::new_v4();

        // Verify the invoice belongs to this org; fetch exchange_rate for FX computation.
        let inv_row: Option<(Uuid, Decimal)> = sqlx::query_as(
            "SELECT id, exchange_rate FROM invoices WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(inv_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        let (_, invoice_exchange_rate) = inv_row.ok_or(DbError::NotFound)?;

        // Compute realized FX gain/loss if payment was recorded at a different rate.
        let realized_fx_amount: i64 = if let Some(pay_rate) = input.exchange_rate {
            if pay_rate != invoice_exchange_rate && !pay_rate.is_zero() {
                let amt = Decimal::from(input.amount);
                let diff = amt - amt * invoice_exchange_rate / pay_rate;
                diff.round().to_i64().unwrap_or(0)
            } else {
                0
            }
        } else {
            0
        };

        sqlx::query(
            "INSERT INTO payments \
             (id, organization_id, invoice_id, amount, payment_date, method, reference, notes, \
              realized_fx_amount) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(inv_uuid)
        .bind(input.amount)
        .bind(input.payment_date)
        .bind(&input.method)
        .bind(&input.reference)
        .bind(&input.notes)
        .bind(realized_fx_amount)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        // Compute total paid and invoice total, update status.
        Self::sync_invoice_status(pool, org_uuid, inv_uuid).await?;

        let row: PaymentRow = sqlx::query_as(&format!(
            "SELECT {PAYMENT_COLS} FROM payments WHERE id = $1"
        ))
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.into())
    }

    pub async fn list_by_invoice(
        pool: &PgPool,
        org_id: &str,
        invoice_id: &str,
    ) -> Result<Vec<Payment>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let inv_uuid = parse_uuid(invoice_id)?;

        let rows: Vec<PaymentRow> = sqlx::query_as(&format!(
            "SELECT {PAYMENT_COLS} \
             FROM payments WHERE organization_id = $1 AND invoice_id = $2 \
             ORDER BY payment_date ASC, created_at ASC"
        ))
        .bind(org_uuid)
        .bind(inv_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows.into_iter().map(Payment::from).collect())
    }

    /// Void a payment, restore invoice status to 'sent' if fully paid/partial.
    pub async fn void(pool: &PgPool, org_id: &str, payment_id: &str) -> Result<Payment, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let pay_uuid = parse_uuid(payment_id)?;

        let rows_affected = sqlx::query(
            "UPDATE payments \
             SET status = 'voided', voided_at = NOW(), updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2 AND status = 'recorded'",
        )
        .bind(pay_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows_affected == 0 {
            let row: Option<PaymentRow> = sqlx::query_as(&format!(
                "SELECT {PAYMENT_COLS} FROM payments WHERE id = $1 AND organization_id = $2"
            ))
            .bind(pay_uuid)
            .bind(org_uuid)
            .fetch_optional(pool)
            .await
            .map_err(map_sqlx_err)?;
            return match row {
                None => Err(DbError::NotFound),
                Some(r) => Err(DbError::Conflict(format!(
                    "payment cannot be voided from status '{}'",
                    r.status
                ))),
            };
        }

        // Re-sync the invoice status after voiding.
        let inv_uuid: (Uuid,) = sqlx::query_as("SELECT invoice_id FROM payments WHERE id = $1")
            .bind(pay_uuid)
            .fetch_one(pool)
            .await
            .map_err(map_sqlx_err)?;
        Self::sync_invoice_status(pool, org_uuid, inv_uuid.0).await?;

        let row: PaymentRow = sqlx::query_as(&format!(
            "SELECT {PAYMENT_COLS} FROM payments WHERE id = $1"
        ))
        .bind(pay_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(row.into())
    }

    pub async fn create_refund(
        pool: &PgPool,
        org_id: &str,
        payment_id: &str,
        input: CreateRefund,
    ) -> Result<Refund, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let pay_uuid = parse_uuid(payment_id)?;

        // Verify payment belongs to this org and is not voided.
        let row: Option<PaymentRow> = sqlx::query_as(&format!(
            "SELECT {PAYMENT_COLS} FROM payments \
             WHERE id = $1 AND organization_id = $2"
        ))
        .bind(pay_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        let payment = row.ok_or(DbError::NotFound)?;
        if payment.status == "voided" {
            return Err(DbError::Conflict("cannot refund a voided payment".into()));
        }
        if input.amount <= 0 || input.amount > payment.amount {
            return Err(DbError::Conflict(
                "refund amount must be positive and not exceed payment amount".into(),
            ));
        }

        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO refunds (id, payment_id, amount, reason, refund_date) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(pay_uuid)
        .bind(input.amount)
        .bind(&input.reason)
        .bind(input.refund_date)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        let refund: RefundRow = sqlx::query_as(
            "SELECT id, payment_id, amount, reason, refund_date, created_at \
             FROM refunds WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(refund.into())
    }

    pub async fn list_refunds(
        pool: &PgPool,
        org_id: &str,
        payment_id: &str,
    ) -> Result<Vec<Refund>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let pay_uuid = parse_uuid(payment_id)?;

        // Verify payment belongs to this org.
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM payments WHERE id = $1 AND organization_id = $2")
                .bind(pay_uuid)
                .bind(org_uuid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;
        if exists.is_none() {
            return Err(DbError::NotFound);
        }

        let rows: Vec<RefundRow> = sqlx::query_as(
            "SELECT id, payment_id, amount, reason, refund_date, created_at \
             FROM refunds WHERE payment_id = $1 ORDER BY created_at ASC",
        )
        .bind(pay_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows.into_iter().map(Refund::from).collect())
    }

    /// Post a realized FX gain/loss journal entry for a payment and link it.
    ///
    /// `ar_account_id` is the AR clearing account; `fx_account_id` is the FX gain/loss GL account.
    /// No-ops if `realized_fx_amount == 0` or a JE is already linked.
    pub async fn post_fx_journal(
        pool: &PgPool,
        org_id: &str,
        payment_id: &str,
        ar_account_id: &str,
        fx_account_id: &str,
    ) -> Result<Payment, DbError> {
        use crate::repos::TransactionRepo;
        use oxidebooks_core::models::{CreateJournalEntry, CreateJournalLine};

        let org_uuid = parse_uuid(org_id)?;
        let pay_uuid = parse_uuid(payment_id)?;

        let row: PaymentRow = sqlx::query_as(&format!(
            "SELECT {PAYMENT_COLS} FROM payments WHERE id = $1 AND organization_id = $2"
        ))
        .bind(pay_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        if row.realized_fx_amount == 0 {
            return Ok(row.into());
        }
        if row.fx_journal_entry_id.is_some() {
            return Err(DbError::Conflict(
                "FX journal entry already posted for this payment".into(),
            ));
        }

        let fx_amount = row.realized_fx_amount.abs();
        let is_gain = row.realized_fx_amount > 0;

        // Gain: Dr AR clearing / Cr FX gain
        // Loss: Dr FX loss / Cr AR clearing
        let lines = if is_gain {
            vec![
                CreateJournalLine {
                    account_id: ar_account_id.to_string(),
                    description: Some("Realized FX gain — AR settlement".to_string()),
                    debit: fx_amount,
                    credit: 0,
                },
                CreateJournalLine {
                    account_id: fx_account_id.to_string(),
                    description: Some("Realized FX gain".to_string()),
                    debit: 0,
                    credit: fx_amount,
                },
            ]
        } else {
            vec![
                CreateJournalLine {
                    account_id: fx_account_id.to_string(),
                    description: Some("Realized FX loss".to_string()),
                    debit: fx_amount,
                    credit: 0,
                },
                CreateJournalLine {
                    account_id: ar_account_id.to_string(),
                    description: Some("Realized FX loss — AR settlement".to_string()),
                    debit: 0,
                    credit: fx_amount,
                },
            ]
        };

        let je_input = CreateJournalEntry {
            date: row.payment_date,
            reference: Some(format!("FX-{}", &payment_id[..8])),
            description: format!(
                "Realized FX {} on payment {}",
                if is_gain { "gain" } else { "loss" },
                &payment_id[..8]
            ),
            lines,
            auto_reversal_date: None,
        };

        let je = TransactionRepo::create_posted(pool, org_id, "system", je_input).await?;
        let je_uuid = parse_uuid(&je.id)?;

        sqlx::query(
            "UPDATE payments SET fx_journal_entry_id = $3, updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2",
        )
        .bind(pay_uuid)
        .bind(org_uuid)
        .bind(je_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        let updated: PaymentRow = sqlx::query_as(&format!(
            "SELECT {PAYMENT_COLS} FROM payments WHERE id = $1"
        ))
        .bind(pay_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(updated.into())
    }

    /// Recompute invoice status based on total non-voided payments vs. invoice line total.
    pub(crate) async fn sync_invoice_status(
        pool: &PgPool,
        org_uuid: Uuid,
        inv_uuid: Uuid,
    ) -> Result<(), DbError> {
        let invoice_total: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(
                 quantity * unit_price / 100
                 - quantity * unit_price / 100 * discount_pct / 10000
                 + (quantity * unit_price / 100 - quantity * unit_price / 100 * discount_pct / 10000)
                   * tax_rate / 10000
             ), 0)::BIGINT \
             FROM invoice_lines WHERE invoice_id = $1",
        )
        .bind(inv_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let paid_total: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(amount), 0)::BIGINT FROM payments \
             WHERE organization_id = $1 AND invoice_id = $2 AND status = 'recorded'",
        )
        .bind(org_uuid)
        .bind(inv_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let new_status = if paid_total.0 >= invoice_total.0 {
            "paid"
        } else if paid_total.0 > 0 {
            "partial"
        } else {
            "sent"
        };

        sqlx::query(
            "UPDATE invoices SET status = $1, updated_at = NOW() \
             WHERE organization_id = $2 AND id = $3 AND status NOT IN ('voided', 'draft')",
        )
        .bind(new_status)
        .bind(org_uuid)
        .bind(inv_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
