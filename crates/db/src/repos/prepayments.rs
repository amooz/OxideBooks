use oxidebooks_core::models::{ApplyPrepayment, CreatePrepayment, Prepayment};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

const COLS: &str =
    "id, organization_id, contact_id, amount, reference, date, applied_amount, status, \
     created_at, updated_at";

#[derive(sqlx::FromRow)]
struct PrepaymentRow {
    id: Uuid,
    organization_id: Uuid,
    contact_id: Uuid,
    amount: i64,
    reference: Option<String>,
    date: Date,
    applied_amount: i64,
    status: String,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<PrepaymentRow> for Prepayment {
    fn from(r: PrepaymentRow) -> Self {
        let remaining_amount = r.amount - r.applied_amount;
        Prepayment {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            contact_id: r.contact_id.to_string(),
            amount: r.amount,
            reference: r.reference,
            date: r.date,
            applied_amount: r.applied_amount,
            remaining_amount,
            status: r.status,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

pub struct PrepaymentRepo;

impl PrepaymentRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        contact_id: Option<&str>,
    ) -> Result<Vec<Prepayment>, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let rows: Vec<PrepaymentRow> = if let Some(cid) = contact_id {
            let contact_uuid = parse_uuid(cid)?;
            sqlx::query_as(&format!(
                "SELECT {COLS} FROM prepayments \
                 WHERE organization_id = $1 AND contact_id = $2 \
                 ORDER BY date DESC, created_at DESC"
            ))
            .bind(org_uuid)
            .bind(contact_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(&format!(
                "SELECT {COLS} FROM prepayments \
                 WHERE organization_id = $1 \
                 ORDER BY date DESC, created_at DESC"
            ))
            .bind(org_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        };

        Ok(rows.into_iter().map(Prepayment::from).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<Prepayment, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let row: PrepaymentRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM prepayments \
             WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        Ok(Prepayment::from(row))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreatePrepayment,
    ) -> Result<Prepayment, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = parse_uuid(&input.contact_id)?;
        let id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO prepayments \
             (id, organization_id, contact_id, amount, reference, date) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(contact_uuid)
        .bind(input.amount)
        .bind(&input.reference)
        .bind(input.date)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn apply(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: ApplyPrepayment,
    ) -> Result<Prepayment, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        // Fetch current state to validate available balance.
        let current = Self::get_by_id(pool, org_id, id).await?;
        if current.status == "voided" {
            return Err(DbError::Conflict("prepayment is voided".into()));
        }
        if current.remaining_amount < input.amount {
            return Err(DbError::Conflict(format!(
                "apply amount {} exceeds remaining balance {}",
                input.amount, current.remaining_amount
            )));
        }

        let new_applied = current.applied_amount + input.amount;
        let new_status = if new_applied >= current.amount {
            "fully_applied"
        } else {
            "partially_applied"
        };

        sqlx::query(
            "UPDATE prepayments \
             SET applied_amount = $3, status = $4, updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .bind(new_applied)
        .bind(new_status)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn void(pool: &PgPool, org_id: &str, id: &str) -> Result<Prepayment, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let rows_affected = sqlx::query(
            "UPDATE prepayments \
             SET status = 'voided', updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2 AND status = 'available'",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows_affected == 0 {
            let p = Self::get_by_id(pool, org_id, id).await?;
            return Err(DbError::Conflict(format!(
                "prepayment cannot be voided from status '{}'",
                p.status
            )));
        }

        Self::get_by_id(pool, org_id, id).await
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
