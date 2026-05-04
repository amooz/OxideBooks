use oxidebooks_core::models::{ConsolidationElimination, CreateConsolidationElimination};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct EliminationRow {
    id: Uuid,
    organization_id: Uuid,
    intercompany_link_id: Option<Uuid>,
    period_start: Date,
    period_end: Date,
    debit_account_id: Uuid,
    credit_account_id: Uuid,
    amount: i64,
    description: String,
    status: String,
    notes: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn from_row(r: EliminationRow) -> ConsolidationElimination {
    ConsolidationElimination {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        intercompany_link_id: r.intercompany_link_id.map(|u| u.to_string()),
        period_start: r.period_start,
        period_end: r.period_end,
        debit_account_id: r.debit_account_id.to_string(),
        credit_account_id: r.credit_account_id.to_string(),
        amount: r.amount,
        description: r.description,
        status: r.status,
        notes: r.notes,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

const COLS: &str = "id, organization_id, intercompany_link_id, period_start, period_end, \
                    debit_account_id, credit_account_id, amount, description, status, notes, \
                    created_at, updated_at";

pub struct ConsolidationEliminationRepo;

impl ConsolidationEliminationRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
    ) -> Result<Vec<ConsolidationElimination>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<EliminationRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM consolidation_eliminations \
             WHERE organization_id = $1 ORDER BY period_end DESC, created_at DESC"
        ))
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn list_for_period(
        pool: &PgPool,
        org_id: &str,
        period_start: Date,
        period_end: Date,
    ) -> Result<Vec<ConsolidationElimination>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<EliminationRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM consolidation_eliminations \
             WHERE organization_id = $1 \
               AND period_start >= $2 AND period_end <= $3 \
               AND status = 'active' \
             ORDER BY created_at ASC"
        ))
        .bind(org_uuid)
        .bind(period_start)
        .bind(period_end)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<ConsolidationElimination, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: EliminationRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM consolidation_eliminations \
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

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateConsolidationElimination,
    ) -> Result<ConsolidationElimination, DbError> {
        if input.amount <= 0 {
            return Err(DbError::Conflict("amount must be positive".into()));
        }
        if input.period_start >= input.period_end {
            return Err(DbError::Conflict(
                "period_start must be before period_end".into(),
            ));
        }
        if input.debit_account_id == input.credit_account_id {
            return Err(DbError::Conflict(
                "debit and credit accounts must differ".into(),
            ));
        }

        let org_uuid = parse_uuid(org_id)?;
        let link_uuid = input
            .intercompany_link_id
            .as_deref()
            .map(parse_uuid)
            .transpose()?;
        let debit_uuid = parse_uuid(&input.debit_account_id)?;
        let credit_uuid = parse_uuid(&input.credit_account_id)?;

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO consolidation_eliminations \
             (organization_id, intercompany_link_id, period_start, period_end, \
              debit_account_id, credit_account_id, amount, description, notes) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING id",
        )
        .bind(org_uuid)
        .bind(link_uuid)
        .bind(input.period_start)
        .bind(input.period_end)
        .bind(debit_uuid)
        .bind(credit_uuid)
        .bind(input.amount)
        .bind(&input.description)
        .bind(&input.notes)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn void(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<ConsolidationElimination, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows = sqlx::query(
            "UPDATE consolidation_eliminations \
             SET status = 'voided', updated_at = NOW() \
             WHERE organization_id = $1 AND id = $2 AND status = 'active'",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if rows == 0 {
            return Err(DbError::Conflict(
                "elimination must be active to void".into(),
            ));
        }
        Self::get_by_id(pool, org_id, id).await
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
