use oxidebooks_core::models::{
    ApplyCreditNote, CreateCreditNote, CreditNote, CreditNoteApplication,
};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct CreditNoteRow {
    id: Uuid,
    organization_id: Uuid,
    contact_id: Option<Uuid>,
    note_date: Date,
    reference: Option<String>,
    description: String,
    amount: i64,
    remaining: i64,
    status: String,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<CreditNoteRow> for CreditNote {
    fn from(r: CreditNoteRow) -> Self {
        CreditNote {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            contact_id: r.contact_id.map(|u| u.to_string()),
            note_date: r.note_date,
            reference: r.reference,
            description: r.description,
            amount: r.amount,
            remaining: r.remaining,
            status: r.status,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ApplicationRow {
    id: Uuid,
    credit_note_id: Uuid,
    invoice_id: Uuid,
    amount_applied: i64,
    applied_at: OffsetDateTime,
}

impl From<ApplicationRow> for CreditNoteApplication {
    fn from(r: ApplicationRow) -> Self {
        CreditNoteApplication {
            id: r.id.to_string(),
            credit_note_id: r.credit_note_id.to_string(),
            invoice_id: r.invoice_id.to_string(),
            amount_applied: r.amount_applied,
            applied_at: r.applied_at,
        }
    }
}

const CN_COLS: &str = "id, organization_id, contact_id, note_date, reference, description, \
     amount, remaining, status, created_at, updated_at";

pub struct CreditNoteRepo;

impl CreditNoteRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<CreditNote>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<CreditNoteRow> = sqlx::query_as(&format!(
            "SELECT {CN_COLS} FROM credit_notes WHERE organization_id = $1 \
             ORDER BY note_date DESC, created_at DESC"
        ))
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(CreditNote::from).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<CreditNote, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: CreditNoteRow = sqlx::query_as(&format!(
            "SELECT {CN_COLS} FROM credit_notes WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(row.into())
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateCreditNote,
    ) -> Result<CreditNote, DbError> {
        if input.amount <= 0 {
            return Err(DbError::Conflict("amount must be positive".into()));
        }
        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = input.contact_id.as_deref().map(parse_uuid).transpose()?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO credit_notes \
             (id, organization_id, contact_id, note_date, reference, description, amount, remaining) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $7)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(contact_uuid)
        .bind(input.note_date)
        .bind(&input.reference)
        .bind(&input.description)
        .bind(input.amount)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn apply(
        pool: &PgPool,
        org_id: &str,
        cn_id: &str,
        input: ApplyCreditNote,
    ) -> Result<CreditNoteApplication, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let cn_uuid = parse_uuid(cn_id)?;
        let inv_uuid = parse_uuid(&input.invoice_id)?;

        if input.amount <= 0 {
            return Err(DbError::Conflict("amount must be positive".into()));
        }

        let cn = Self::get_by_id(pool, org_id, cn_id).await?;
        if cn.status == "voided" || cn.remaining == 0 {
            return Err(DbError::Conflict(
                "credit note is fully applied or voided".into(),
            ));
        }
        if input.amount > cn.remaining {
            return Err(DbError::Conflict(format!(
                "amount exceeds remaining credit ({} available)",
                cn.remaining
            )));
        }

        // Verify invoice belongs to this org.
        let inv_exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM invoices WHERE organization_id = $1 AND id = $2")
                .bind(org_uuid)
                .bind(inv_uuid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;
        if inv_exists.is_none() {
            return Err(DbError::NotFound);
        }

        let app_id = Uuid::new_v4();
        let new_remaining = cn.remaining - input.amount;
        let new_status = if new_remaining == 0 {
            "applied"
        } else {
            "partial"
        };

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        sqlx::query(
            "INSERT INTO credit_note_applications (id, credit_note_id, invoice_id, amount_applied) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(app_id)
        .bind(cn_uuid)
        .bind(inv_uuid)
        .bind(input.amount)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        sqlx::query(
            "UPDATE credit_notes SET remaining = $1, status = $2, updated_at = NOW() WHERE id = $3",
        )
        .bind(new_remaining)
        .bind(new_status)
        .bind(cn_uuid)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;

        let row: ApplicationRow = sqlx::query_as(
            "SELECT id, credit_note_id, invoice_id, amount_applied, applied_at \
             FROM credit_note_applications WHERE id = $1",
        )
        .bind(app_id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.into())
    }

    pub async fn void(pool: &PgPool, org_id: &str, id: &str) -> Result<CreditNote, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows = sqlx::query(
            "UPDATE credit_notes SET status = 'voided', updated_at = NOW() \
             WHERE organization_id = $1 AND id = $2 AND status IN ('open','partial')",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if rows == 0 {
            let cn = Self::get_by_id(pool, org_id, id).await?;
            return Err(DbError::Conflict(format!(
                "cannot void credit note with status '{}'",
                cn.status
            )));
        }
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn list_applications(
        pool: &PgPool,
        org_id: &str,
        cn_id: &str,
    ) -> Result<Vec<CreditNoteApplication>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let cn_uuid = parse_uuid(cn_id)?;
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM credit_notes WHERE organization_id = $1 AND id = $2")
                .bind(org_uuid)
                .bind(cn_uuid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;
        if exists.is_none() {
            return Err(DbError::NotFound);
        }
        let rows: Vec<ApplicationRow> = sqlx::query_as(
            "SELECT id, credit_note_id, invoice_id, amount_applied, applied_at \
             FROM credit_note_applications WHERE credit_note_id = $1 \
             ORDER BY applied_at ASC",
        )
        .bind(cn_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(CreditNoteApplication::from).collect())
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
