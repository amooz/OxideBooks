use oxidebooks_core::models::{CreatePaymentTerms, PaymentTerms, UpdatePaymentTerms};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

const COLS: &str = "id, organization_id, name, net_days, discount_days, discount_pct, is_default, \
     created_at, updated_at";

#[derive(sqlx::FromRow)]
struct TermsRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    net_days: i32,
    discount_days: Option<i32>,
    discount_pct: i64,
    is_default: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<TermsRow> for PaymentTerms {
    fn from(r: TermsRow) -> Self {
        PaymentTerms {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            name: r.name,
            net_days: r.net_days,
            discount_days: r.discount_days,
            discount_pct: r.discount_pct,
            is_default: r.is_default,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

pub struct PaymentTermsRepo;

impl PaymentTermsRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<PaymentTerms>, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let rows: Vec<TermsRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM payment_terms \
             WHERE organization_id = $1 ORDER BY name"
        ))
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows.into_iter().map(PaymentTerms::from).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<PaymentTerms, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let row: TermsRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM payment_terms \
             WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        Ok(PaymentTerms::from(row))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreatePaymentTerms,
    ) -> Result<PaymentTerms, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id = Uuid::new_v4();

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        // If this term is the new default, unset any existing default first.
        if input.is_default {
            sqlx::query(
                "UPDATE payment_terms SET is_default = FALSE, updated_at = NOW() \
                 WHERE organization_id = $1 AND is_default = TRUE",
            )
            .bind(org_uuid)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        sqlx::query(
            "INSERT INTO payment_terms \
             (id, organization_id, name, net_days, discount_days, discount_pct, is_default) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(&input.name)
        .bind(input.net_days)
        .bind(input.discount_days)
        .bind(input.discount_pct)
        .bind(input.is_default)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdatePaymentTerms,
    ) -> Result<PaymentTerms, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        if input.is_default == Some(true) {
            sqlx::query(
                "UPDATE payment_terms SET is_default = FALSE, updated_at = NOW() \
                 WHERE organization_id = $1 AND is_default = TRUE AND id != $2",
            )
            .bind(org_uuid)
            .bind(id_uuid)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        sqlx::query(
            "UPDATE payment_terms SET \
             name          = COALESCE($3, name),
             net_days      = COALESCE($4, net_days),
             discount_days = COALESCE($5, discount_days),
             discount_pct  = COALESCE($6, discount_pct),
             is_default    = COALESCE($7, is_default),
             updated_at    = NOW()
             WHERE id = $1 AND organization_id = $2",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .bind(&input.name)
        .bind(input.net_days)
        .bind(input.discount_days)
        .bind(input.discount_pct)
        .bind(input.is_default)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        sqlx::query("DELETE FROM payment_terms WHERE id = $1 AND organization_id = $2")
            .bind(id_uuid)
            .bind(org_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;

        Ok(())
    }

    /// Compute the due date for an invoice given a start date and these terms.
    pub fn compute_due_date(issue_date: time::Date, net_days: i32) -> time::Date {
        issue_date + time::Duration::days(net_days as i64)
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
