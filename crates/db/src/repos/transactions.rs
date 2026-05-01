use oxidebooks_core::models::{
    CreateJournalEntry, JournalEntry, JournalEntryStatus, JournalLine,
};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct EntryRow {
    id: Uuid,
    organization_id: Uuid,
    date: Date,
    reference: Option<String>,
    description: String,
    status: String,
    created_by: Uuid,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct LineRow {
    id: Uuid,
    journal_entry_id: Uuid,
    account_id: Uuid,
    description: Option<String>,
    debit: i64,
    credit: i64,
}

pub struct TransactionRepo;

impl TransactionRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<JournalEntry>, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let rows: Vec<EntryRow> = sqlx::query_as(
            "SELECT id, organization_id, date, reference, description, status, \
             created_by, created_at, updated_at \
             FROM journal_entries WHERE organization_id = $1 \
             ORDER BY date DESC, created_at DESC",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut entries = Vec::with_capacity(rows.len());
        for r in rows {
            let lines = Self::fetch_lines(pool, r.id).await?;
            entries.push(entry_from_row(r, lines));
        }
        Ok(entries)
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<JournalEntry, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let row: EntryRow = sqlx::query_as(
            "SELECT id, organization_id, date, reference, description, status, \
             created_by, created_at, updated_at \
             FROM journal_entries WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        let lines = Self::fetch_lines(pool, row.id).await?;
        Ok(entry_from_row(row, lines))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        user_id: &str,
        input: CreateJournalEntry,
    ) -> Result<JournalEntry, DbError> {
        input.validate()?;

        let org_uuid = parse_uuid(org_id)?;
        let user_uuid = parse_uuid(user_id)?;
        let id = Uuid::new_v4();

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        sqlx::query(
            "INSERT INTO journal_entries \
             (id, organization_id, date, reference, description, status, created_by) \
             VALUES ($1, $2, $3, $4, $5, 'posted', $6)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(input.date)
        .bind(&input.reference)
        .bind(&input.description)
        .bind(user_uuid)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        for line in &input.lines {
            let line_id = Uuid::new_v4();
            let acct_uuid = parse_uuid(&line.account_id)?;
            sqlx::query(
                "INSERT INTO journal_lines \
                 (id, journal_entry_id, account_id, description, debit, credit) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(line_id)
            .bind(id)
            .bind(acct_uuid)
            .bind(&line.description)
            .bind(line.debit)
            .bind(line.credit)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    async fn fetch_lines(pool: &PgPool, entry_id: Uuid) -> Result<Vec<JournalLine>, DbError> {
        let rows: Vec<LineRow> = sqlx::query_as(
            "SELECT id, journal_entry_id, account_id, description, debit, credit \
             FROM journal_lines WHERE journal_entry_id = $1",
        )
        .bind(entry_id)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows
            .into_iter()
            .map(|r| JournalLine {
                id: r.id.to_string(),
                journal_entry_id: r.journal_entry_id.to_string(),
                account_id: r.account_id.to_string(),
                description: r.description,
                debit: r.debit,
                credit: r.credit,
            })
            .collect())
    }
}

fn entry_from_row(r: EntryRow, lines: Vec<JournalLine>) -> JournalEntry {
    JournalEntry {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        date: r.date,
        reference: r.reference,
        description: r.description,
        status: match r.status.as_str() {
            "draft" => JournalEntryStatus::Draft,
            "voided" => JournalEntryStatus::Voided,
            _ => JournalEntryStatus::Posted,
        },
        lines,
        created_by: r.created_by.to_string(),
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
