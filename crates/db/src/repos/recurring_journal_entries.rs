use oxidebooks_core::models::{
    CreateRecurringJournalEntry, RecurringJournalEntry, RecurringJournalEntryLine,
    UpdateRecurringJournalEntry,
};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct RjeRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    description: Option<String>,
    frequency: String,
    next_date: Date,
    end_date: Option<Date>,
    is_active: bool,
    auto_post: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct LineRow {
    id: Uuid,
    recurring_journal_entry_id: Uuid,
    account_id: Uuid,
    description: Option<String>,
    debit: i64,
    credit: i64,
}

const RJE_COLS: &str = "id, organization_id, name, description, frequency, \
    next_date, end_date, is_active, auto_post, created_at, updated_at";

pub struct RecurringJournalEntryRepo;

impl RecurringJournalEntryRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<RecurringJournalEntry>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<RjeRow> = sqlx::query_as(&format!(
            "SELECT {RJE_COLS} FROM recurring_journal_entries \
             WHERE organization_id = $1 ORDER BY next_date ASC, name ASC"
        ))
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut result = Vec::with_capacity(rows.len());
        for r in rows {
            let lines = Self::fetch_lines(pool, r.id).await?;
            result.push(rje_from_row(r, lines));
        }
        Ok(result)
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<RecurringJournalEntry, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: RjeRow = sqlx::query_as(&format!(
            "SELECT {RJE_COLS} FROM recurring_journal_entries \
             WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        let lines = Self::fetch_lines(pool, row.id).await?;
        Ok(rje_from_row(row, lines))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateRecurringJournalEntry,
    ) -> Result<RecurringJournalEntry, DbError> {
        validate_frequency(&input.frequency)?;
        validate_lines(&input.lines)?;

        let org_uuid = parse_uuid(org_id)?;
        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        let rje_id: Uuid = sqlx::query_scalar(
            "INSERT INTO recurring_journal_entries \
             (organization_id, name, description, frequency, next_date, end_date, auto_post) \
             VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id",
        )
        .bind(org_uuid)
        .bind(&input.name)
        .bind(&input.description)
        .bind(&input.frequency)
        .bind(input.next_date)
        .bind(input.end_date)
        .bind(input.auto_post)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        for line in &input.lines {
            let acct_uuid = parse_uuid(&line.account_id)?;
            sqlx::query(
                "INSERT INTO recurring_journal_entry_lines \
                 (recurring_journal_entry_id, account_id, description, debit, credit) \
                 VALUES ($1,$2,$3,$4,$5)",
            )
            .bind(rje_id)
            .bind(acct_uuid)
            .bind(&line.description)
            .bind(line.debit)
            .bind(line.credit)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, &rje_id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateRecurringJournalEntry,
    ) -> Result<RecurringJournalEntry, DbError> {
        if let Some(ref f) = input.frequency {
            validate_frequency(f)?;
        }
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let rows = sqlx::query(
            "UPDATE recurring_journal_entries SET \
             name       = COALESCE($3, name), \
             description= COALESCE($4, description), \
             frequency  = COALESCE($5, frequency), \
             next_date  = COALESCE($6, next_date), \
             end_date   = COALESCE($7, end_date), \
             is_active  = COALESCE($8, is_active), \
             auto_post  = COALESCE($9, auto_post), \
             updated_at = NOW() \
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .bind(&input.name)
        .bind(&input.description)
        .bind(&input.frequency)
        .bind(input.next_date)
        .bind(input.end_date)
        .bind(input.is_active)
        .bind(input.auto_post)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows == 0 {
            return Err(DbError::NotFound);
        }
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows = sqlx::query(
            "DELETE FROM recurring_journal_entries WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if rows == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    /// Post (generate) the next journal entry for this recurring template.
    /// Advances `next_date` by one frequency period. If `end_date` is passed
    /// the entry is deactivated after posting.
    pub async fn post_next(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<RecurringJournalEntry, DbError> {
        let rje = Self::get_by_id(pool, org_id, id).await?;
        if !rje.is_active {
            return Err(DbError::Conflict(
                "recurring journal entry is inactive".into(),
            ));
        }

        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        // Build the journal entry lines for the transaction.
        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        let je_id: Uuid = sqlx::query_scalar(
            "INSERT INTO journal_entries (organization_id, entry_date, description, status) \
             VALUES ($1, $2, $3, 'posted') RETURNING id",
        )
        .bind(org_uuid)
        .bind(rje.next_date)
        .bind(&rje.name)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        for line in &rje.lines {
            let acct_uuid = parse_uuid(&line.account_id)?;
            sqlx::query(
                "INSERT INTO journal_entry_lines \
                 (journal_entry_id, account_id, description, debit, credit) \
                 VALUES ($1,$2,$3,$4,$5)",
            )
            .bind(je_id)
            .bind(acct_uuid)
            .bind(&line.description)
            .bind(line.debit)
            .bind(line.credit)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        // Advance next_date.
        let new_next_date = advance_date(rje.next_date, &rje.frequency);
        let still_active = rje.end_date.is_none_or(|end| new_next_date <= end);

        sqlx::query(
            "UPDATE recurring_journal_entries SET \
             next_date = $3, is_active = $4, updated_at = NOW() \
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .bind(new_next_date)
        .bind(still_active)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, id).await
    }

    async fn fetch_lines(
        pool: &PgPool,
        rje_id: Uuid,
    ) -> Result<Vec<RecurringJournalEntryLine>, DbError> {
        let rows: Vec<LineRow> = sqlx::query_as(
            "SELECT id, recurring_journal_entry_id, account_id, description, debit, credit \
             FROM recurring_journal_entry_lines \
             WHERE recurring_journal_entry_id = $1",
        )
        .bind(rje_id)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows
            .into_iter()
            .map(|r| RecurringJournalEntryLine {
                id: r.id.to_string(),
                recurring_journal_entry_id: r.recurring_journal_entry_id.to_string(),
                account_id: r.account_id.to_string(),
                description: r.description,
                debit: r.debit,
                credit: r.credit,
            })
            .collect())
    }
}

fn rje_from_row(r: RjeRow, lines: Vec<RecurringJournalEntryLine>) -> RecurringJournalEntry {
    RecurringJournalEntry {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        name: r.name,
        description: r.description,
        frequency: r.frequency,
        next_date: r.next_date,
        end_date: r.end_date,
        is_active: r.is_active,
        auto_post: r.auto_post,
        lines,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

fn advance_date(date: Date, frequency: &str) -> Date {
    match frequency {
        "daily" => date + time::Duration::days(1),
        "weekly" => date + time::Duration::days(7),
        "biweekly" => date + time::Duration::days(14),
        "monthly" => {
            let (y, m, d) = (date.year(), date.month() as u8, date.day());
            let (ny, nm) = if m == 12 {
                (y + 1, time::Month::January)
            } else {
                (y, time::Month::try_from(m + 1).unwrap())
            };
            let max_day = days_in_month(ny, nm);
            Date::from_calendar_date(ny, nm, d.min(max_day)).unwrap()
        }
        "quarterly" => {
            let (y, m, d) = (date.year(), date.month() as u8, date.day());
            let total_months = m as i32 + 3;
            let ny = y + (total_months - 1) / 12;
            let nm = time::Month::try_from(((total_months - 1) % 12 + 1) as u8).unwrap();
            let max_day = days_in_month(ny, nm);
            Date::from_calendar_date(ny, nm, d.min(max_day)).unwrap()
        }
        "yearly" => {
            let nm = date.month();
            let ny = date.year() + 1;
            let max_day = days_in_month(ny, nm);
            Date::from_calendar_date(ny, nm, date.day().min(max_day)).unwrap()
        }
        _ => date + time::Duration::days(30),
    }
}

fn days_in_month(year: i32, month: time::Month) -> u8 {
    match month {
        time::Month::January
        | time::Month::March
        | time::Month::May
        | time::Month::July
        | time::Month::August
        | time::Month::October
        | time::Month::December => 31,
        time::Month::April | time::Month::June | time::Month::September | time::Month::November => {
            30
        }
        time::Month::February => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
    }
}

fn validate_frequency(f: &str) -> Result<(), DbError> {
    match f {
        "daily" | "weekly" | "biweekly" | "monthly" | "quarterly" | "yearly" => Ok(()),
        other => Err(DbError::Conflict(format!("invalid frequency '{other}'"))),
    }
}

fn validate_lines(
    lines: &[oxidebooks_core::models::CreateRecurringJournalEntryLine],
) -> Result<(), DbError> {
    if lines.len() < 2 {
        return Err(DbError::Conflict(
            "journal entry must have at least two lines".into(),
        ));
    }
    let total_debit: i64 = lines.iter().map(|l| l.debit).sum();
    let total_credit: i64 = lines.iter().map(|l| l.credit).sum();
    if total_debit != total_credit {
        return Err(DbError::Conflict(format!(
            "debits ({total_debit}) must equal credits ({total_credit})"
        )));
    }
    Ok(())
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
