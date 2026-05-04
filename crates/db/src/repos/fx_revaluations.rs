use oxidebooks_core::models::{CreateFxRevaluation, FxRevaluation};
use rust_decimal::Decimal;
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};
use crate::repos::TransactionRepo;
use oxidebooks_core::models::{CreateJournalEntry, CreateJournalLine};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct RevRow {
    id: Uuid,
    organization_id: Uuid,
    revaluation_date: Date,
    currency: String,
    rate: Decimal,
    net_gain_loss: i64,
    journal_entry_id: Option<Uuid>,
    notes: Option<String>,
    created_at: OffsetDateTime,
}

fn from_row(r: RevRow) -> FxRevaluation {
    FxRevaluation {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        revaluation_date: r.revaluation_date,
        currency: r.currency,
        rate: r.rate,
        net_gain_loss: r.net_gain_loss,
        journal_entry_id: r.journal_entry_id.map(|u| u.to_string()),
        notes: r.notes,
        created_at: r.created_at,
    }
}

pub struct FxRevaluationRepo;

impl FxRevaluationRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<FxRevaluation>, DbError> {
        let org = parse_uuid(org_id)?;
        let rows: Vec<RevRow> = sqlx::query_as(
            "SELECT id, organization_id, revaluation_date, currency, rate, net_gain_loss,
                    journal_entry_id, notes, created_at
             FROM fx_revaluations
             WHERE organization_id = $1
             ORDER BY revaluation_date DESC, created_at DESC",
        )
        .bind(org)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Compute unrealized FX gain/loss on open AR/AP invoices denominated in `currency`
    /// at the given rate, post a journal entry, and record the revaluation.
    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateFxRevaluation,
    ) -> Result<FxRevaluation, DbError> {
        let org = parse_uuid(org_id)?;

        // Sum of (invoice_amount - paid_amount) for open invoices in this currency
        // Gain/loss = outstanding_foreign * (new_rate - booked_rate)
        // We approximate: net_gain_loss = Σ (amount_due * (new_rate - exchange_rate)) for AR
        //                                - Σ (amount_due * (new_rate - exchange_rate)) for AP
        let ar_gain_loss: Option<i64> = sqlx::query_scalar(
            "SELECT COALESCE(SUM(
                ROUND((total_amount - paid_amount) * ($2::NUMERIC - COALESCE(exchange_rate, 1)))
             ), 0)::BIGINT
             FROM invoices
             WHERE organization_id = $1
               AND currency = $3
               AND invoice_type = 'invoice'
               AND status NOT IN ('voided','draft')
               AND paid_amount < total_amount",
        )
        .bind(org)
        .bind(input.rate)
        .bind(&input.currency)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let ap_gain_loss: Option<i64> = sqlx::query_scalar(
            "SELECT COALESCE(SUM(
                ROUND((total_amount - paid_amount) * ($2::NUMERIC - COALESCE(exchange_rate, 1)))
             ), 0)::BIGINT
             FROM invoices
             WHERE organization_id = $1
               AND currency = $3
               AND invoice_type = 'bill'
               AND status NOT IN ('voided','draft')
               AND paid_amount < total_amount",
        )
        .bind(org)
        .bind(input.rate)
        .bind(&input.currency)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let net = ar_gain_loss.unwrap_or(0) - ap_gain_loss.unwrap_or(0);

        // Look up unrealized FX gain/loss accounts (system accounts by code)
        let gain_acct: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM accounts WHERE organization_id = $1 AND code = 'FX_GAIN_UNREALIZED' LIMIT 1",
        )
        .bind(org)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        let loss_acct: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM accounts WHERE organization_id = $1 AND code = 'FX_LOSS_UNREALIZED' LIMIT 1",
        )
        .bind(org)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        // Post journal entry if accounts exist and there's a non-zero gain/loss
        let journal_entry_id: Option<Uuid> = if net != 0 {
            if let (Some((gain_id,)), Some((loss_id,))) = (gain_acct, loss_acct) {
                let ar_acct: Option<(Uuid,)> = sqlx::query_as(
                    "SELECT id FROM accounts WHERE organization_id = $1 AND code = 'AR' LIMIT 1",
                )
                .bind(org)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;

                if let Some((ar_id,)) = ar_acct {
                    let (debit_acct, credit_acct) = if net > 0 {
                        (ar_id, gain_id)
                    } else {
                        (loss_id, ar_id)
                    };
                    let abs_net = net.unsigned_abs() as i64;
                    let je = CreateJournalEntry {
                        date: input.revaluation_date,
                        reference: None,
                        description: format!("FX revaluation {} @ {}", input.currency, input.rate),
                        lines: vec![
                            CreateJournalLine {
                                account_id: debit_acct.to_string(),
                                debit: abs_net,
                                credit: 0,
                                description: None,
                            },
                            CreateJournalLine {
                                account_id: credit_acct.to_string(),
                                debit: 0,
                                credit: abs_net,
                                description: None,
                            },
                        ],
                        auto_reversal_date: None,
                    };
                    let entry = TransactionRepo::create(pool, org_id, "system", je).await?;
                    parse_uuid(&entry.id).ok()
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO fx_revaluations
                (organization_id, revaluation_date, currency, rate, net_gain_loss,
                 journal_entry_id, notes)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING id",
        )
        .bind(org)
        .bind(input.revaluation_date)
        .bind(&input.currency)
        .bind(input.rate)
        .bind(net)
        .bind(journal_entry_id)
        .bind(&input.notes)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: RevRow = sqlx::query_as(
            "SELECT id, organization_id, revaluation_date, currency, rate, net_gain_loss,
                    journal_entry_id, notes, created_at
             FROM fx_revaluations WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(from_row(row))
    }
}
