use oxidebooks_core::models::{CreateJournalEntry, JournalEntry, JournalEntryStatus, JournalLine};
use oxidebooks_core::pagination::{encode_cursor, PageParams};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

const ENTRY_COLS: &str = "id, organization_id, date, reference, description, status, \
                          created_by, reversal_of, submitted_by, submitted_at, \
                          approved_by, approved_at, created_at, updated_at";

#[derive(sqlx::FromRow)]
struct EntryRow {
    id: Uuid,
    organization_id: Uuid,
    date: Date,
    reference: Option<String>,
    description: String,
    status: String,
    created_by: Uuid,
    reversal_of: Option<Uuid>,
    submitted_by: Option<Uuid>,
    submitted_at: Option<OffsetDateTime>,
    approved_by: Option<Uuid>,
    approved_at: Option<OffsetDateTime>,
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
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        page: &PageParams,
    ) -> Result<(Vec<JournalEntry>, Option<String>), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let limit = page.limit_clamped();
        let cursor = page.decode_cursor();

        let rows: Vec<EntryRow> = if let Some(c) = cursor {
            let cursor_ts = time::OffsetDateTime::parse(
                &c.created_at,
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(|_| DbError::Conflict("invalid cursor".into()))?;
            let cursor_id = parse_uuid(&c.id)?;
            sqlx::query_as(&format!(
                "SELECT {ENTRY_COLS} FROM journal_entries \
                 WHERE organization_id = $1 AND (created_at, id) > ($2, $3) \
                 ORDER BY created_at ASC, id ASC LIMIT $4"
            ))
            .bind(org_uuid)
            .bind(cursor_ts)
            .bind(cursor_id)
            .bind(limit + 1)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(&format!(
                "SELECT {ENTRY_COLS} FROM journal_entries \
                 WHERE organization_id = $1 \
                 ORDER BY created_at ASC, id ASC LIMIT $2"
            ))
            .bind(org_uuid)
            .bind(limit + 1)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        };

        let has_next = rows.len() as i64 > limit;
        let mut rows = rows;
        if has_next {
            rows.pop();
        }
        let next_cursor = if has_next {
            rows.last()
                .map(|r| encode_cursor(r.created_at, &r.id.to_string()))
        } else {
            None
        };
        let mut entries = Vec::with_capacity(rows.len());
        for r in rows {
            let lines = Self::fetch_lines(pool, r.id).await?;
            entries.push(entry_from_row(r, lines));
        }
        Ok((entries, next_cursor))
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<JournalEntry, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let row: EntryRow = sqlx::query_as(&format!(
            "SELECT {ENTRY_COLS} FROM journal_entries \
             WHERE organization_id = $1 AND id = $2"
        ))
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
        Self::insert_entry(pool, org_id, user_id, "draft", input).await
    }

    /// Create a journal entry pre-posted (no approval workflow).
    /// Use for system-generated entries: invoices, payroll, opening balances.
    pub async fn create_posted(
        pool: &PgPool,
        org_id: &str,
        user_id: &str,
        input: CreateJournalEntry,
    ) -> Result<JournalEntry, DbError> {
        Self::insert_entry(pool, org_id, user_id, "posted", input).await
    }

    async fn insert_entry(
        pool: &PgPool,
        org_id: &str,
        user_id: &str,
        status: &str,
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
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(input.date)
        .bind(&input.reference)
        .bind(&input.description)
        .bind(status)
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

    pub async fn get_opening_balance(
        pool: &PgPool,
        org_id: &str,
    ) -> Result<Option<JournalEntry>, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let row: Option<EntryRow> = sqlx::query_as(&format!(
            "SELECT {ENTRY_COLS} FROM journal_entries \
             WHERE organization_id = $1 AND reference = 'OPENING_BALANCE' \
             ORDER BY created_at DESC LIMIT 1"
        ))
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        match row {
            None => Ok(None),
            Some(r) => {
                let lines = Self::fetch_lines(pool, r.id).await?;
                Ok(Some(entry_from_row(r, lines)))
            }
        }
    }

    pub async fn submit(
        pool: &PgPool,
        org_id: &str,
        user_id: &str,
        id: &str,
    ) -> Result<JournalEntry, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let user_uuid = parse_uuid(user_id)?;

        let rows_affected = sqlx::query(
            "UPDATE journal_entries \
             SET status = 'submitted', submitted_by = $3, submitted_at = NOW(), updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2 AND status = 'draft'",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .bind(user_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows_affected == 0 {
            let entry = Self::get_by_id(pool, org_id, id).await?;
            return Err(DbError::Conflict(format!(
                "journal entry cannot be submitted from status '{}'",
                entry.status
            )));
        }

        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn approve(
        pool: &PgPool,
        org_id: &str,
        user_id: &str,
        id: &str,
    ) -> Result<JournalEntry, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let user_uuid = parse_uuid(user_id)?;

        let rows_affected = sqlx::query(
            "UPDATE journal_entries \
             SET status = 'posted', approved_by = $3, approved_at = NOW(), updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2 AND status = 'submitted'",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .bind(user_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows_affected == 0 {
            let entry = Self::get_by_id(pool, org_id, id).await?;
            return Err(DbError::Conflict(format!(
                "journal entry cannot be approved from status '{}'",
                entry.status
            )));
        }

        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn void(pool: &PgPool, org_id: &str, id: &str) -> Result<JournalEntry, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let rows_affected = sqlx::query(
            "UPDATE journal_entries \
             SET status = 'voided', updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2 AND status = 'posted'",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows_affected == 0 {
            // Either not found or not in 'posted' state — distinguish by fetching.
            let entry = Self::get_by_id(pool, org_id, id).await?;
            return Err(DbError::Conflict(format!(
                "journal entry cannot be voided from status '{}'",
                entry.status
            )));
        }

        // If this entry is linked to an invoice, void that invoice too.
        sqlx::query(
            "UPDATE invoices \
             SET status = 'voided', updated_at = NOW() \
             WHERE journal_entry_id = $1 AND organization_id = $2",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, id).await
    }

    /// Create a reversing journal entry (debits↔credits swapped) for a posted entry.
    pub async fn reverse(
        pool: &PgPool,
        org_id: &str,
        user_id: &str,
        id: &str,
        reversal_date: Option<Date>,
    ) -> Result<JournalEntry, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let user_uuid = parse_uuid(user_id)?;
        let id_uuid = parse_uuid(id)?;

        let original: EntryRow = sqlx::query_as(&format!(
            "SELECT {ENTRY_COLS} FROM journal_entries \
             WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        if original.status != "posted" {
            return Err(DbError::Conflict(format!(
                "cannot reverse a journal entry with status '{}'",
                original.status
            )));
        }

        // Prevent double-reversal: check if a reversal already exists.
        let already_reversed: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM journal_entries WHERE reversal_of = $1 LIMIT 1")
                .bind(id_uuid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;

        if already_reversed.is_some() {
            return Err(DbError::Conflict(
                "this journal entry has already been reversed".into(),
            ));
        }

        let original_lines = Self::fetch_lines(pool, original.id).await?;
        let effective_date = reversal_date.unwrap_or(original.date);
        let reversal_id = Uuid::new_v4();
        let description = format!("Reversal of: {}", original.description);

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        sqlx::query(
            "INSERT INTO journal_entries \
             (id, organization_id, date, reference, description, status, created_by, reversal_of) \
             VALUES ($1, $2, $3, $4, $5, 'posted', $6, $7)",
        )
        .bind(reversal_id)
        .bind(org_uuid)
        .bind(effective_date)
        .bind(&original.reference)
        .bind(&description)
        .bind(user_uuid)
        .bind(id_uuid)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        for line in &original_lines {
            let line_id = Uuid::new_v4();
            let acct_uuid = parse_uuid(&line.account_id)?;
            // Swap debit/credit for the reversal.
            sqlx::query(
                "INSERT INTO journal_lines \
                 (id, journal_entry_id, account_id, description, debit, credit) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(line_id)
            .bind(reversal_id)
            .bind(acct_uuid)
            .bind(&line.description)
            .bind(line.credit)
            .bind(line.debit)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &reversal_id.to_string()).await
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
            "submitted" => JournalEntryStatus::Submitted,
            "voided" => JournalEntryStatus::Voided,
            _ => JournalEntryStatus::Posted,
        },
        lines,
        created_by: r.created_by.to_string(),
        reversal_of: r.reversal_of.map(|u| u.to_string()),
        submitted_by: r.submitted_by.map(|u| u.to_string()),
        submitted_at: r.submitted_at,
        approved_by: r.approved_by.map(|u| u.to_string()),
        approved_at: r.approved_at,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
