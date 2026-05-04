use oxidebooks_core::models::{CreateExpense, Expense, ExpenseStatus, UpdateExpense};
use oxidebooks_core::pagination::{encode_cursor, PageParams};
use sqlx::PgPool;
use std::str::FromStr;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct ExpenseRow {
    id: Uuid,
    organization_id: Uuid,
    user_id: Uuid,
    expense_date: Date,
    amount: i64,
    currency: String,
    category: String,
    description: String,
    account_id: Option<Uuid>,
    project_id: Option<Uuid>,
    status: String,
    is_billable: bool,
    billable_contact_id: Option<Uuid>,
    billed_invoice_id: Option<Uuid>,
    receipt_url: Option<String>,
    notes: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn from_row(r: ExpenseRow) -> Expense {
    Expense {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        user_id: r.user_id.to_string(),
        expense_date: r.expense_date,
        amount: r.amount,
        currency: r.currency,
        category: r.category,
        description: r.description,
        account_id: r.account_id.map(|u| u.to_string()),
        project_id: r.project_id.map(|u| u.to_string()),
        status: ExpenseStatus::from_str(&r.status).unwrap_or(ExpenseStatus::Draft),
        is_billable: r.is_billable,
        billable_contact_id: r.billable_contact_id.map(|u| u.to_string()),
        billed_invoice_id: r.billed_invoice_id.map(|u| u.to_string()),
        receipt_url: r.receipt_url,
        notes: r.notes,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

const COLS: &str = "id, organization_id, user_id, expense_date, amount, currency, category, \
                    description, account_id, project_id, status, is_billable, \
                    billable_contact_id, billed_invoice_id, receipt_url, notes, \
                    created_at, updated_at";

pub struct ExpenseRepo;

impl ExpenseRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        page: &PageParams,
        user_id_filter: Option<&str>,
        status_filter: Option<&str>,
    ) -> Result<(Vec<Expense>, Option<String>), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let limit = page.limit_clamped();
        let cursor = page.decode_cursor();
        let user_uuid = user_id_filter.map(parse_uuid).transpose()?;

        let rows: Vec<ExpenseRow> = if let Some(c) = cursor {
            let cursor_ts = time::OffsetDateTime::parse(
                &c.created_at,
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(|_| DbError::Conflict("invalid cursor".into()))?;
            let cursor_id = parse_uuid(&c.id)?;
            sqlx::query_as(&format!(
                "SELECT {COLS} FROM expenses \
                 WHERE organization_id = $1 \
                   AND ($2::uuid IS NULL OR user_id = $2) \
                   AND ($3::text IS NULL OR status = $3) \
                   AND (created_at, id) > ($4, $5) \
                 ORDER BY created_at ASC, id ASC LIMIT $6"
            ))
            .bind(org_uuid)
            .bind(user_uuid)
            .bind(status_filter)
            .bind(cursor_ts)
            .bind(cursor_id)
            .bind(limit + 1)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(&format!(
                "SELECT {COLS} FROM expenses \
                 WHERE organization_id = $1 \
                   AND ($2::uuid IS NULL OR user_id = $2) \
                   AND ($3::text IS NULL OR status = $3) \
                 ORDER BY created_at ASC, id ASC LIMIT $4"
            ))
            .bind(org_uuid)
            .bind(user_uuid)
            .bind(status_filter)
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
        Ok((rows.into_iter().map(from_row).collect(), next_cursor))
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<Expense, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: ExpenseRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM expenses WHERE organization_id = $1 AND id = $2"
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
        input: CreateExpense,
    ) -> Result<Expense, DbError> {
        if input.amount <= 0 {
            return Err(DbError::Conflict("amount must be positive".into()));
        }
        let org_uuid = parse_uuid(org_id)?;
        let user_uuid = parse_uuid(user_id)?;
        let acct_uuid = input.account_id.as_deref().map(parse_uuid).transpose()?;
        let proj_uuid = input.project_id.as_deref().map(parse_uuid).transpose()?;
        let contact_uuid = input
            .billable_contact_id
            .as_deref()
            .map(parse_uuid)
            .transpose()?;

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO expenses \
             (organization_id, user_id, expense_date, amount, currency, category, description, \
              account_id, project_id, is_billable, billable_contact_id, receipt_url, notes) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13) RETURNING id",
        )
        .bind(org_uuid)
        .bind(user_uuid)
        .bind(input.expense_date)
        .bind(input.amount)
        .bind(&input.currency)
        .bind(&input.category)
        .bind(&input.description)
        .bind(acct_uuid)
        .bind(proj_uuid)
        .bind(input.is_billable.unwrap_or(false))
        .bind(contact_uuid)
        .bind(&input.receipt_url)
        .bind(&input.notes)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateExpense,
    ) -> Result<Expense, DbError> {
        let current = Self::get_by_id(pool, org_id, id).await?;
        if current.status != ExpenseStatus::Draft {
            return Err(DbError::Conflict(
                "only draft expenses can be edited".into(),
            ));
        }
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let acct_uuid = input.account_id.as_deref().map(parse_uuid).transpose()?;

        let billable_contact_uuid = input
            .billable_contact_id
            .as_deref()
            .map(parse_uuid)
            .transpose()?;

        sqlx::query(
            "UPDATE expenses SET \
             expense_date         = COALESCE($1, expense_date), \
             amount               = COALESCE($2, amount), \
             category             = COALESCE($3, category), \
             description          = COALESCE($4, description), \
             account_id           = COALESCE($5, account_id), \
             is_billable          = COALESCE($6, is_billable), \
             billable_contact_id  = COALESCE($7, billable_contact_id), \
             receipt_url          = COALESCE($8, receipt_url), \
             notes                = COALESCE($9, notes), \
             updated_at           = NOW() \
             WHERE id = $10 AND organization_id = $11",
        )
        .bind(input.expense_date)
        .bind(input.amount)
        .bind(input.category)
        .bind(input.description)
        .bind(acct_uuid)
        .bind(input.is_billable)
        .bind(billable_contact_uuid)
        .bind(input.receipt_url)
        .bind(input.notes)
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, id).await
    }

    /// Returns unbilled billable expenses for a contact in the org.
    pub async fn list_billable(
        pool: &PgPool,
        org_id: &str,
        contact_id: &str,
    ) -> Result<Vec<Expense>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = parse_uuid(contact_id)?;
        let rows: Vec<ExpenseRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM expenses \
             WHERE organization_id = $1 \
               AND billable_contact_id = $2 \
               AND is_billable = TRUE \
               AND billed_invoice_id IS NULL \
             ORDER BY expense_date ASC, id ASC"
        ))
        .bind(org_uuid)
        .bind(contact_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Marks a set of expense IDs as billed against the given invoice.
    pub async fn mark_billed(
        pool: &PgPool,
        org_id: &str,
        expense_ids: &[Uuid],
        invoice_id: Uuid,
    ) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        sqlx::query(
            "UPDATE expenses SET billed_invoice_id = $1, updated_at = NOW() \
             WHERE organization_id = $2 AND id = ANY($3)",
        )
        .bind(invoice_id)
        .bind(org_uuid)
        .bind(expense_ids)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }

    pub async fn transition(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        new_status: ExpenseStatus,
    ) -> Result<Expense, DbError> {
        let current = Self::get_by_id(pool, org_id, id).await?;
        if !current.status.can_transition_to(&new_status) {
            return Err(DbError::Conflict(format!(
                "cannot transition expense from '{}' to '{}'",
                current.status, new_status
            )));
        }
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        sqlx::query(
            "UPDATE expenses SET status = $1, updated_at = NOW() \
             WHERE id = $2 AND organization_id = $3",
        )
        .bind(new_status.to_string())
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, id).await
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
