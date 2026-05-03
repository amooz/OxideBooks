use oxidebooks_core::models::{
    BankAccount, BankTransaction, CreateBankAccount, ImportBankTransaction, MatchTransaction,
    ReconciliationSummary, UpdateBankAccount,
};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct BankAccountRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    account_number: Option<String>,
    institution: Option<String>,
    currency: String,
    current_balance: i64,
    gl_account_id: Option<Uuid>,
    is_active: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct BankTxnRow {
    id: Uuid,
    bank_account_id: Uuid,
    organization_id: Uuid,
    txn_date: Date,
    description: String,
    amount: i64,
    txn_type: String,
    status: String,
    reference: Option<String>,
    matched_payment_id: Option<Uuid>,
    matched_expense_id: Option<Uuid>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn acct_from_row(r: BankAccountRow) -> BankAccount {
    BankAccount {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        name: r.name,
        account_number: r.account_number,
        institution: r.institution,
        currency: r.currency,
        current_balance: r.current_balance,
        gl_account_id: r.gl_account_id.map(|u| u.to_string()),
        is_active: r.is_active,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

fn txn_from_row(r: BankTxnRow) -> BankTransaction {
    BankTransaction {
        id: r.id.to_string(),
        bank_account_id: r.bank_account_id.to_string(),
        organization_id: r.organization_id.to_string(),
        txn_date: r.txn_date,
        description: r.description,
        amount: r.amount,
        txn_type: r.txn_type,
        status: r.status,
        reference: r.reference,
        matched_payment_id: r.matched_payment_id.map(|u| u.to_string()),
        matched_expense_id: r.matched_expense_id.map(|u| u.to_string()),
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

pub struct BankRepo;

impl BankRepo {
    // ── Bank accounts ──────────────────────────────────────────────────────────

    pub async fn list_accounts(pool: &PgPool, org_id: &str) -> Result<Vec<BankAccount>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<BankAccountRow> = sqlx::query_as(
            "SELECT id, organization_id, name, account_number, institution, currency, \
             current_balance, gl_account_id, is_active, created_at, updated_at \
             FROM bank_accounts WHERE organization_id = $1 ORDER BY name",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(acct_from_row).collect())
    }

    pub async fn get_account(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<BankAccount, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: BankAccountRow = sqlx::query_as(
            "SELECT id, organization_id, name, account_number, institution, currency, \
             current_balance, gl_account_id, is_active, created_at, updated_at \
             FROM bank_accounts WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(acct_from_row(row))
    }

    pub async fn create_account(
        pool: &PgPool,
        org_id: &str,
        input: CreateBankAccount,
    ) -> Result<BankAccount, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let gl_uuid = input.gl_account_id.as_deref().map(parse_uuid).transpose()?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO bank_accounts (organization_id, name, account_number, institution, currency, gl_account_id) \
             VALUES ($1,$2,$3,$4,$5,$6) RETURNING id",
        )
        .bind(org_uuid)
        .bind(&input.name)
        .bind(&input.account_number)
        .bind(&input.institution)
        .bind(&input.currency)
        .bind(gl_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Self::get_account(pool, org_id, &id.to_string()).await
    }

    pub async fn update_account(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateBankAccount,
    ) -> Result<BankAccount, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let gl_uuid = input.gl_account_id.as_deref().map(parse_uuid).transpose()?;
        let n = sqlx::query(
            "UPDATE bank_accounts SET \
             name           = COALESCE($1, name), \
             account_number = COALESCE($2, account_number), \
             institution    = COALESCE($3, institution), \
             gl_account_id  = COALESCE($4, gl_account_id), \
             is_active      = COALESCE($5, is_active), \
             updated_at     = NOW() \
             WHERE id = $6 AND organization_id = $7",
        )
        .bind(input.name)
        .bind(input.account_number)
        .bind(input.institution)
        .bind(gl_uuid)
        .bind(input.is_active)
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::NotFound);
        }
        Self::get_account(pool, org_id, id).await
    }

    // ── Bank transactions ──────────────────────────────────────────────────────

    pub async fn list_transactions(
        pool: &PgPool,
        org_id: &str,
        account_id: &str,
        status: Option<&str>,
    ) -> Result<Vec<BankTransaction>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let acct_uuid = parse_uuid(account_id)?;
        let rows: Vec<BankTxnRow> = sqlx::query_as(
            "SELECT id, bank_account_id, organization_id, txn_date, description, amount, \
             txn_type, status, reference, matched_payment_id, matched_expense_id, \
             created_at, updated_at \
             FROM bank_transactions \
             WHERE organization_id = $1 AND bank_account_id = $2 \
               AND ($3::text IS NULL OR status = $3) \
             ORDER BY txn_date DESC, created_at DESC",
        )
        .bind(org_uuid)
        .bind(acct_uuid)
        .bind(status)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(txn_from_row).collect())
    }

    pub async fn import_transactions(
        pool: &PgPool,
        org_id: &str,
        account_id: &str,
        txns: Vec<ImportBankTransaction>,
    ) -> Result<usize, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let acct_uuid = parse_uuid(account_id)?;

        // Verify ownership
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM bank_accounts WHERE id = $1 AND organization_id = $2)",
        )
        .bind(acct_uuid)
        .bind(org_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        if !exists {
            return Err(DbError::NotFound);
        }

        let count = txns.len();
        for t in txns {
            sqlx::query(
                "INSERT INTO bank_transactions \
                 (bank_account_id, organization_id, txn_date, description, amount, txn_type, reference) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7)",
            )
            .bind(acct_uuid)
            .bind(org_uuid)
            .bind(t.txn_date)
            .bind(&t.description)
            .bind(t.amount)
            .bind(&t.txn_type)
            .bind(&t.reference)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;
        }
        Ok(count)
    }

    pub async fn match_transaction(
        pool: &PgPool,
        org_id: &str,
        txn_id: &str,
        input: MatchTransaction,
    ) -> Result<BankTransaction, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let txn_uuid = parse_uuid(txn_id)?;
        let payment_uuid = input.payment_id.as_deref().map(parse_uuid).transpose()?;
        let expense_uuid = input.expense_id.as_deref().map(parse_uuid).transpose()?;

        if payment_uuid.is_none() && expense_uuid.is_none() {
            return Err(DbError::Conflict(
                "must provide payment_id or expense_id".into(),
            ));
        }

        let n = sqlx::query(
            "UPDATE bank_transactions SET \
             status = 'matched', \
             matched_payment_id = COALESCE($1, matched_payment_id), \
             matched_expense_id = COALESCE($2, matched_expense_id), \
             updated_at = NOW() \
             WHERE id = $3 AND organization_id = $4 AND status = 'unmatched'",
        )
        .bind(payment_uuid)
        .bind(expense_uuid)
        .bind(txn_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if n == 0 {
            return Err(DbError::Conflict(
                "transaction not found or already matched".into(),
            ));
        }

        let row: BankTxnRow = sqlx::query_as(
            "SELECT id, bank_account_id, organization_id, txn_date, description, amount, \
             txn_type, status, reference, matched_payment_id, matched_expense_id, \
             created_at, updated_at \
             FROM bank_transactions WHERE id = $1",
        )
        .bind(txn_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(txn_from_row(row))
    }

    pub async fn exclude_transaction(
        pool: &PgPool,
        org_id: &str,
        txn_id: &str,
    ) -> Result<BankTransaction, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let txn_uuid = parse_uuid(txn_id)?;

        let n = sqlx::query(
            "UPDATE bank_transactions SET status = 'excluded', updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2 AND status = 'unmatched'",
        )
        .bind(txn_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if n == 0 {
            return Err(DbError::Conflict(
                "transaction not found or not unmatched".into(),
            ));
        }

        let row: BankTxnRow = sqlx::query_as(
            "SELECT id, bank_account_id, organization_id, txn_date, description, amount, \
             txn_type, status, reference, matched_payment_id, matched_expense_id, \
             created_at, updated_at \
             FROM bank_transactions WHERE id = $1",
        )
        .bind(txn_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(txn_from_row(row))
    }

    pub async fn reconciliation_summary(
        pool: &PgPool,
        org_id: &str,
        account_id: &str,
    ) -> Result<ReconciliationSummary, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let acct_uuid = parse_uuid(account_id)?;

        #[derive(sqlx::FromRow)]
        struct SummaryRow {
            unmatched_count: i64,
            matched_count: i64,
            excluded_count: i64,
            unmatched_total: i64,
        }

        let row: SummaryRow = sqlx::query_as(
            "SELECT \
               COUNT(*) FILTER (WHERE status = 'unmatched') AS unmatched_count, \
               COUNT(*) FILTER (WHERE status = 'matched')   AS matched_count, \
               COUNT(*) FILTER (WHERE status = 'excluded')  AS excluded_count, \
               COALESCE(SUM(amount) FILTER (WHERE status = 'unmatched'), 0) AS unmatched_total \
             FROM bank_transactions \
             WHERE bank_account_id = $1 AND organization_id = $2",
        )
        .bind(acct_uuid)
        .bind(org_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(ReconciliationSummary {
            bank_account_id: account_id.to_string(),
            unmatched_count: row.unmatched_count,
            matched_count: row.matched_count,
            excluded_count: row.excluded_count,
            unmatched_total: row.unmatched_total,
        })
    }
}
