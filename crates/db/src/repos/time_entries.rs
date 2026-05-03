use oxidebooks_core::models::{
    BillTimeEntries, CreateTimeEntry, TimeEntry, TimeSummaryRow, UpdateTimeEntry,
};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct TimeEntryRow {
    id: Uuid,
    organization_id: Uuid,
    user_id: Uuid,
    project_id: Option<Uuid>,
    contact_id: Option<Uuid>,
    entry_date: Date,
    minutes: i32,
    description: String,
    hourly_rate: i64,
    is_billable: bool,
    invoice_line_id: Option<Uuid>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct SummaryRow {
    user_id: Uuid,
    project_id: Option<Uuid>,
    total_minutes: i64,
    billable_minutes: i64,
    billable_amount: i64,
}

fn from_row(r: TimeEntryRow) -> TimeEntry {
    TimeEntry {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        user_id: r.user_id.to_string(),
        project_id: r.project_id.map(|u| u.to_string()),
        contact_id: r.contact_id.map(|u| u.to_string()),
        entry_date: r.entry_date,
        minutes: r.minutes,
        description: r.description,
        hourly_rate: r.hourly_rate,
        is_billable: r.is_billable,
        invoice_line_id: r.invoice_line_id.map(|u| u.to_string()),
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

const COLS: &str = "id, organization_id, user_id, project_id, contact_id, entry_date, \
                    minutes, description, hourly_rate, is_billable, invoice_line_id, \
                    created_at, updated_at";

pub struct TimeEntryRepo;

impl TimeEntryRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        user_id: Option<&str>,
        project_id: Option<&str>,
        from: Option<Date>,
        to: Option<Date>,
    ) -> Result<Vec<TimeEntry>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let user_uuid = user_id.map(parse_uuid).transpose()?;
        let proj_uuid = project_id.map(parse_uuid).transpose()?;

        let rows: Vec<TimeEntryRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM time_entries \
             WHERE organization_id = $1 \
               AND ($2::uuid IS NULL OR user_id = $2) \
               AND ($3::uuid IS NULL OR project_id = $3) \
               AND ($4::date IS NULL OR entry_date >= $4) \
               AND ($5::date IS NULL OR entry_date <= $5) \
             ORDER BY entry_date DESC, created_at DESC"
        ))
        .bind(org_uuid)
        .bind(user_uuid)
        .bind(proj_uuid)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<TimeEntry, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: TimeEntryRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM time_entries WHERE organization_id = $1 AND id = $2"
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
        user_id: &str,
        input: CreateTimeEntry,
    ) -> Result<TimeEntry, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let user_uuid = parse_uuid(user_id)?;
        let proj_uuid = input.project_id.as_deref().map(parse_uuid).transpose()?;
        let contact_uuid = input.contact_id.as_deref().map(parse_uuid).transpose()?;

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO time_entries \
             (organization_id, user_id, project_id, contact_id, entry_date, minutes, \
              description, hourly_rate, is_billable) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING id",
        )
        .bind(org_uuid)
        .bind(user_uuid)
        .bind(proj_uuid)
        .bind(contact_uuid)
        .bind(input.entry_date)
        .bind(input.minutes)
        .bind(&input.description)
        .bind(input.hourly_rate)
        .bind(input.is_billable)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateTimeEntry,
    ) -> Result<TimeEntry, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let proj_uuid = input.project_id.as_deref().map(parse_uuid).transpose()?;

        let n = sqlx::query(
            "UPDATE time_entries SET \
             project_id  = COALESCE($1, project_id), \
             minutes     = COALESCE($2, minutes), \
             description = COALESCE($3, description), \
             hourly_rate = COALESCE($4, hourly_rate), \
             is_billable = COALESCE($5, is_billable), \
             updated_at  = NOW() \
             WHERE id = $6 AND organization_id = $7",
        )
        .bind(proj_uuid)
        .bind(input.minutes)
        .bind(input.description)
        .bind(input.hourly_rate)
        .bind(input.is_billable)
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if n == 0 {
            return Err(DbError::NotFound);
        }
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n = sqlx::query("DELETE FROM time_entries WHERE id = $1 AND organization_id = $2")
            .bind(id_uuid)
            .bind(org_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?
            .rows_affected();
        if n == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    /// Convert unbilled time entries into invoice lines on an existing invoice.
    pub async fn bill_entries(
        pool: &PgPool,
        org_id: &str,
        input: BillTimeEntries,
    ) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let invoice_uuid = parse_uuid(&input.invoice_id)?;
        let acct_uuid = input.account_id.as_deref().map(parse_uuid).transpose()?;

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        // Verify invoice belongs to org
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM invoices WHERE id = $1 AND organization_id = $2)",
        )
        .bind(invoice_uuid)
        .bind(org_uuid)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;
        if !exists {
            return Err(DbError::NotFound);
        }

        let sort_base: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM invoice_lines WHERE invoice_id = $1",
        )
        .bind(invoice_uuid)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        for (i, entry_id) in input.entry_ids.iter().enumerate() {
            let entry_uuid = parse_uuid(entry_id)?;

            // Fetch entry details
            let row: Option<TimeEntryRow> = sqlx::query_as(&format!(
                "SELECT {COLS} FROM time_entries \
                 WHERE id = $1 AND organization_id = $2 AND invoice_line_id IS NULL"
            ))
            .bind(entry_uuid)
            .bind(org_uuid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;

            let entry = match row {
                Some(r) => from_row(r),
                None => continue,
            };

            let amount = entry.minutes as i64 * entry.hourly_rate / 60;

            let line_id: Uuid = sqlx::query_scalar(
                "INSERT INTO invoice_lines \
                 (invoice_id, account_id, description, quantity, unit_price, tax_rate, sort_order) \
                 VALUES ($1,$2,$3,1,$4,0,$5) RETURNING id",
            )
            .bind(invoice_uuid)
            .bind(acct_uuid)
            .bind(&entry.description)
            .bind(amount)
            .bind(sort_base + i as i32)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;

            sqlx::query(
                "UPDATE time_entries SET invoice_line_id = $1, updated_at = NOW() WHERE id = $2",
            )
            .bind(line_id)
            .bind(entry_uuid)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        sqlx::query("UPDATE invoices SET updated_at = NOW() WHERE id = $1")
            .bind(invoice_uuid)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(())
    }

    pub async fn time_summary(
        pool: &PgPool,
        org_id: &str,
        from: Option<Date>,
        to: Option<Date>,
    ) -> Result<Vec<TimeSummaryRow>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<SummaryRow> = sqlx::query_as(
            "SELECT \
               user_id, \
               project_id, \
               SUM(minutes)::BIGINT AS total_minutes, \
               SUM(CASE WHEN is_billable THEN minutes ELSE 0 END)::BIGINT AS billable_minutes, \
               SUM(CASE WHEN is_billable THEN minutes::BIGINT * hourly_rate / 60 ELSE 0 END) \
                 AS billable_amount \
             FROM time_entries \
             WHERE organization_id = $1 \
               AND ($2::date IS NULL OR entry_date >= $2) \
               AND ($3::date IS NULL OR entry_date <= $3) \
             GROUP BY user_id, project_id \
             ORDER BY user_id",
        )
        .bind(org_uuid)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows
            .into_iter()
            .map(|r| TimeSummaryRow {
                user_id: r.user_id.to_string(),
                project_id: r.project_id.map(|u| u.to_string()),
                total_minutes: r.total_minutes,
                billable_minutes: r.billable_minutes,
                billable_amount: r.billable_amount,
            })
            .collect())
    }
}
