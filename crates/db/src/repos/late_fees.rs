use oxidebooks_core::models::{LateFee, LateFeeRule, UpsertLateFeeRule};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct RuleRow {
    id: Uuid,
    organization_id: Uuid,
    grace_days: i32,
    fee_type: String,
    amount: i64,
    created_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct FeeRow {
    id: Uuid,
    invoice_id: Uuid,
    organization_id: Uuid,
    amount: i64,
    applied_at: OffsetDateTime,
}

fn rule_from_row(r: RuleRow) -> LateFeeRule {
    LateFeeRule {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        grace_days: r.grace_days,
        fee_type: r.fee_type,
        amount: r.amount,
        created_at: r.created_at,
    }
}

fn fee_from_row(r: FeeRow) -> LateFee {
    LateFee {
        id: r.id.to_string(),
        invoice_id: r.invoice_id.to_string(),
        organization_id: r.organization_id.to_string(),
        amount: r.amount,
        applied_at: r.applied_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

pub struct LateFeeRepo;

impl LateFeeRepo {
    pub async fn get_rule(pool: &PgPool, org_id: &str) -> Result<Option<LateFeeRule>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let row: Option<RuleRow> = sqlx::query_as(
            "SELECT id, organization_id, grace_days, fee_type, amount, created_at \
             FROM late_fee_rules WHERE organization_id = $1",
        )
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(row.map(rule_from_row))
    }

    pub async fn upsert_rule(
        pool: &PgPool,
        org_id: &str,
        input: UpsertLateFeeRule,
    ) -> Result<LateFeeRule, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO late_fee_rules (organization_id, grace_days, fee_type, amount) \
             VALUES ($1,$2,$3,$4) \
             ON CONFLICT (organization_id) DO UPDATE \
               SET grace_days = EXCLUDED.grace_days, \
                   fee_type   = EXCLUDED.fee_type, \
                   amount     = EXCLUDED.amount \
             RETURNING id",
        )
        .bind(org_uuid)
        .bind(input.grace_days)
        .bind(&input.fee_type)
        .bind(input.amount)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: RuleRow = sqlx::query_as(
            "SELECT id, organization_id, grace_days, fee_type, amount, created_at \
             FROM late_fee_rules WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rule_from_row(row))
    }

    pub async fn record_fee(
        pool: &PgPool,
        org_id: &str,
        invoice_id: &str,
        amount: i64,
    ) -> Result<LateFee, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let inv_uuid = parse_uuid(invoice_id)?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO late_fees (invoice_id, organization_id, amount) \
             VALUES ($1,$2,$3) RETURNING id",
        )
        .bind(inv_uuid)
        .bind(org_uuid)
        .bind(amount)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: FeeRow = sqlx::query_as(
            "SELECT id, invoice_id, organization_id, amount, applied_at \
             FROM late_fees WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(fee_from_row(row))
    }

    pub async fn list_for_invoice(
        pool: &PgPool,
        org_id: &str,
        invoice_id: &str,
    ) -> Result<Vec<LateFee>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let inv_uuid = parse_uuid(invoice_id)?;
        let rows: Vec<FeeRow> = sqlx::query_as(
            "SELECT id, invoice_id, organization_id, amount, applied_at \
             FROM late_fees WHERE invoice_id = $1 AND organization_id = $2 \
             ORDER BY applied_at DESC",
        )
        .bind(inv_uuid)
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(fee_from_row).collect())
    }
}
