use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use oxidebooks_core::models::{
    CreateExpenseClaim, CreateExpenseClaimLine, ExpenseClaim, ExpenseClaimLine, UpdateExpenseClaim,
};

use crate::error::DbError;

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

const CLAIM_COLS: &str = "id, organization_id, claimant_id, title, description, status,
    submitted_at, reviewed_at, reviewer_id, reviewer_notes, reimbursed_at,
    currency_code, total_amount, created_at, updated_at";

const LINE_COLS: &str =
    "id, claim_id, date, description, amount, category, receipt_url, account_id, sort_order";

#[derive(sqlx::FromRow)]
struct ClaimRow {
    id: Uuid,
    organization_id: Uuid,
    claimant_id: String,
    title: String,
    description: Option<String>,
    status: String,
    submitted_at: Option<OffsetDateTime>,
    reviewed_at: Option<OffsetDateTime>,
    reviewer_id: Option<String>,
    reviewer_notes: Option<String>,
    reimbursed_at: Option<OffsetDateTime>,
    currency_code: String,
    total_amount: i64,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct LineRow {
    id: Uuid,
    claim_id: Uuid,
    date: time::Date,
    description: String,
    amount: i64,
    category: Option<String>,
    receipt_url: Option<String>,
    account_id: Option<String>,
    sort_order: i32,
}

impl From<LineRow> for ExpenseClaimLine {
    fn from(r: LineRow) -> Self {
        ExpenseClaimLine {
            id: r.id.to_string(),
            claim_id: r.claim_id.to_string(),
            date: r.date,
            description: r.description,
            amount: r.amount,
            category: r.category,
            receipt_url: r.receipt_url,
            account_id: r.account_id,
            sort_order: r.sort_order,
        }
    }
}

fn to_claim(r: ClaimRow, lines: Vec<ExpenseClaimLine>) -> ExpenseClaim {
    ExpenseClaim {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        claimant_id: r.claimant_id,
        title: r.title,
        description: r.description,
        status: r.status,
        submitted_at: r.submitted_at,
        reviewed_at: r.reviewed_at,
        reviewer_id: r.reviewer_id,
        reviewer_notes: r.reviewer_notes,
        reimbursed_at: r.reimbursed_at,
        currency_code: r.currency_code,
        total_amount: r.total_amount,
        lines,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

async fn fetch_lines(pool: &PgPool, claim_id: Uuid) -> Result<Vec<ExpenseClaimLine>, DbError> {
    let rows = sqlx::query_as::<_, LineRow>(&format!(
        "SELECT {LINE_COLS} FROM expense_claim_lines WHERE claim_id = $1 ORDER BY sort_order, id"
    ))
    .bind(claim_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub struct ExpenseClaimRepo;

impl ExpenseClaimRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        status: Option<&str>,
    ) -> Result<Vec<ExpenseClaim>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows = if let Some(s) = status {
            sqlx::query_as::<_, ClaimRow>(&format!(
                "SELECT {CLAIM_COLS} FROM expense_claims
                 WHERE organization_id = $1 AND status = $2
                 ORDER BY created_at DESC"
            ))
            .bind(org_uuid)
            .bind(s)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, ClaimRow>(&format!(
                "SELECT {CLAIM_COLS} FROM expense_claims
                 WHERE organization_id = $1
                 ORDER BY created_at DESC"
            ))
            .bind(org_uuid)
            .fetch_all(pool)
            .await?
        };

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let lines = fetch_lines(pool, row.id).await?;
            out.push(to_claim(row, lines));
        }
        Ok(out)
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<ExpenseClaim, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let claim_uuid = parse_uuid(id)?;
        let row = sqlx::query_as::<_, ClaimRow>(&format!(
            "SELECT {CLAIM_COLS} FROM expense_claims
             WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(claim_uuid)
        .fetch_optional(pool)
        .await?
        .ok_or(DbError::NotFound)?;

        let lines = fetch_lines(pool, row.id).await?;
        Ok(to_claim(row, lines))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateExpenseClaim,
    ) -> Result<ExpenseClaim, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let total: i64 = input.lines.iter().map(|l| l.amount).sum();
        let currency = input.currency_code.unwrap_or_else(|| "USD".to_string());

        let row = sqlx::query_as::<_, ClaimRow>(&format!(
            "INSERT INTO expense_claims
                (organization_id, claimant_id, title, description, currency_code, total_amount)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING {CLAIM_COLS}"
        ))
        .bind(org_uuid)
        .bind(&input.claimant_id)
        .bind(&input.title)
        .bind(&input.description)
        .bind(&currency)
        .bind(total)
        .fetch_one(pool)
        .await?;

        let claim_id = row.id;
        Self::insert_lines(pool, claim_id, &input.lines).await?;
        let lines = fetch_lines(pool, claim_id).await?;
        Ok(to_claim(row, lines))
    }

    async fn insert_lines(
        pool: &PgPool,
        claim_id: Uuid,
        lines: &[CreateExpenseClaimLine],
    ) -> Result<(), DbError> {
        for (i, line) in lines.iter().enumerate() {
            sqlx::query(
                "INSERT INTO expense_claim_lines
                    (claim_id, date, description, amount, category, receipt_url, account_id, sort_order)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(claim_id)
            .bind(line.date)
            .bind(&line.description)
            .bind(line.amount)
            .bind(&line.category)
            .bind(&line.receipt_url)
            .bind(&line.account_id)
            .bind(if line.sort_order != 0 { line.sort_order } else { i as i32 })
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateExpenseClaim,
    ) -> Result<ExpenseClaim, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let claim_uuid = parse_uuid(id)?;
        let row = sqlx::query_as::<_, ClaimRow>(&format!(
            "UPDATE expense_claims
             SET title = COALESCE($3, title),
                 description = COALESCE($4, description),
                 updated_at = now()
             WHERE organization_id = $1 AND id = $2 AND status = 'draft'
             RETURNING {CLAIM_COLS}"
        ))
        .bind(org_uuid)
        .bind(claim_uuid)
        .bind(&input.title)
        .bind(&input.description)
        .fetch_optional(pool)
        .await?
        .ok_or(DbError::NotFound)?;

        let lines = fetch_lines(pool, row.id).await?;
        Ok(to_claim(row, lines))
    }

    pub async fn submit(pool: &PgPool, org_id: &str, id: &str) -> Result<ExpenseClaim, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let claim_uuid = parse_uuid(id)?;
        let row = sqlx::query_as::<_, ClaimRow>(&format!(
            "UPDATE expense_claims
             SET status = 'submitted', submitted_at = now(), updated_at = now()
             WHERE organization_id = $1 AND id = $2 AND status = 'draft'
             RETURNING {CLAIM_COLS}"
        ))
        .bind(org_uuid)
        .bind(claim_uuid)
        .fetch_optional(pool)
        .await?
        .ok_or(DbError::NotFound)?;

        let lines = fetch_lines(pool, row.id).await?;
        Ok(to_claim(row, lines))
    }

    pub async fn approve(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        reviewer_id: &str,
        notes: Option<String>,
    ) -> Result<ExpenseClaim, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let claim_uuid = parse_uuid(id)?;
        let row = sqlx::query_as::<_, ClaimRow>(&format!(
            "UPDATE expense_claims
             SET status = 'approved', reviewed_at = now(), reviewer_id = $3,
                 reviewer_notes = $4, updated_at = now()
             WHERE organization_id = $1 AND id = $2 AND status = 'submitted'
             RETURNING {CLAIM_COLS}"
        ))
        .bind(org_uuid)
        .bind(claim_uuid)
        .bind(reviewer_id)
        .bind(notes)
        .fetch_optional(pool)
        .await?
        .ok_or(DbError::NotFound)?;

        let lines = fetch_lines(pool, row.id).await?;
        Ok(to_claim(row, lines))
    }

    pub async fn reject(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        reviewer_id: &str,
        notes: Option<String>,
    ) -> Result<ExpenseClaim, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let claim_uuid = parse_uuid(id)?;
        let row = sqlx::query_as::<_, ClaimRow>(&format!(
            "UPDATE expense_claims
             SET status = 'rejected', reviewed_at = now(), reviewer_id = $3,
                 reviewer_notes = $4, updated_at = now()
             WHERE organization_id = $1 AND id = $2 AND status = 'submitted'
             RETURNING {CLAIM_COLS}"
        ))
        .bind(org_uuid)
        .bind(claim_uuid)
        .bind(reviewer_id)
        .bind(notes)
        .fetch_optional(pool)
        .await?
        .ok_or(DbError::NotFound)?;

        let lines = fetch_lines(pool, row.id).await?;
        Ok(to_claim(row, lines))
    }

    pub async fn mark_reimbursed(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<ExpenseClaim, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let claim_uuid = parse_uuid(id)?;
        let row = sqlx::query_as::<_, ClaimRow>(&format!(
            "UPDATE expense_claims
             SET status = 'reimbursed', reimbursed_at = now(), updated_at = now()
             WHERE organization_id = $1 AND id = $2 AND status = 'approved'
             RETURNING {CLAIM_COLS}"
        ))
        .bind(org_uuid)
        .bind(claim_uuid)
        .fetch_optional(pool)
        .await?
        .ok_or(DbError::NotFound)?;

        let lines = fetch_lines(pool, row.id).await?;
        Ok(to_claim(row, lines))
    }
}
