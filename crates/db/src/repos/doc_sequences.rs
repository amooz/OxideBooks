use oxidebooks_core::models::{DocSequence, ResetDocSequence, UpsertDocSequence};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::DbError;

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct SeqRow {
    id: Uuid,
    organization_id: Uuid,
    doc_type: String,
    prefix: String,
    next_number: i64,
    pad_length: i32,
    created_at: time::OffsetDateTime,
    updated_at: time::OffsetDateTime,
}

fn from_row(r: SeqRow) -> DocSequence {
    DocSequence {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        doc_type: r.doc_type,
        prefix: r.prefix,
        next_number: r.next_number,
        pad_length: r.pad_length,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

pub struct DocSequenceRepo;

impl DocSequenceRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<DocSequence>, DbError> {
        let org = parse_uuid(org_id)?;
        let rows = sqlx::query_as::<_, SeqRow>(
            "SELECT id, organization_id, doc_type, prefix, next_number, pad_length,
                    created_at, updated_at
             FROM doc_sequences
             WHERE organization_id = $1
             ORDER BY doc_type",
        )
        .bind(org)
        .fetch_all(pool)
        .await?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn upsert(
        pool: &PgPool,
        org_id: &str,
        input: UpsertDocSequence,
    ) -> Result<DocSequence, DbError> {
        let org = parse_uuid(org_id)?;
        let valid_types = [
            "invoice",
            "bill",
            "credit_note",
            "purchase_order",
            "quote",
            "expense_report",
            "payment",
        ];
        if !valid_types.contains(&input.doc_type.as_str()) {
            return Err(DbError::Conflict("invalid doc_type".into()));
        }
        let row = sqlx::query_as::<_, SeqRow>(
            "INSERT INTO doc_sequences (organization_id, doc_type, prefix, pad_length)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (organization_id, doc_type) DO UPDATE
                SET prefix     = COALESCE(EXCLUDED.prefix, doc_sequences.prefix),
                    pad_length = COALESCE(EXCLUDED.pad_length, doc_sequences.pad_length),
                    updated_at = now()
             RETURNING id, organization_id, doc_type, prefix, next_number, pad_length,
                       created_at, updated_at",
        )
        .bind(org)
        .bind(&input.doc_type)
        .bind(input.prefix.unwrap_or_default())
        .bind(input.pad_length.unwrap_or(4))
        .fetch_one(pool)
        .await?;
        Ok(from_row(row))
    }

    pub async fn reset(
        pool: &PgPool,
        org_id: &str,
        doc_type: &str,
        input: ResetDocSequence,
    ) -> Result<DocSequence, DbError> {
        let org = parse_uuid(org_id)?;
        if input.next_number < 1 {
            return Err(DbError::Conflict("next_number must be >= 1".into()));
        }
        let row = sqlx::query_as::<_, SeqRow>(
            "UPDATE doc_sequences
             SET next_number = $3, updated_at = now()
             WHERE organization_id = $1 AND doc_type = $2
             RETURNING id, organization_id, doc_type, prefix, next_number, pad_length,
                       created_at, updated_at",
        )
        .bind(org)
        .bind(doc_type)
        .bind(input.next_number)
        .fetch_optional(pool)
        .await?
        .ok_or(DbError::NotFound)?;
        Ok(from_row(row))
    }

    /// Atomically claim the next number and return the formatted document number string.
    pub async fn next(pool: &PgPool, org_id: &str, doc_type: &str) -> Result<String, DbError> {
        let org = parse_uuid(org_id)?;
        let row: (String, i64, i32) = sqlx::query_as(
            "UPDATE doc_sequences
             SET next_number = next_number + 1, updated_at = now()
             WHERE organization_id = $1 AND doc_type = $2
             RETURNING prefix, next_number - 1, pad_length",
        )
        .bind(org)
        .bind(doc_type)
        .fetch_optional(pool)
        .await?
        .ok_or(DbError::NotFound)?;

        let (prefix, n, pad) = row;
        Ok(format!("{}{:0>width$}", prefix, n, width = pad as usize))
    }
}
