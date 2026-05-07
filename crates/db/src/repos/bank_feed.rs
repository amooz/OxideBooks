use oxidebooks_core::models::{
    BankFeedAutoMatchResult, BankFeedTransaction, ImportBankFeed, MatchBankFeedTransaction,
};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct FeedRow {
    id: Uuid,
    organization_id: Uuid,
    bank_account_id: Uuid,
    txn_date: Date,
    description: String,
    amount: i64,
    txn_type: String,
    reference: Option<String>,
    source: String,
    status: String,
    matched_txn_id: Option<Uuid>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<FeedRow> for BankFeedTransaction {
    fn from(r: FeedRow) -> Self {
        BankFeedTransaction {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            bank_account_id: r.bank_account_id.to_string(),
            txn_date: r.txn_date,
            description: r.description,
            amount: r.amount,
            txn_type: r.txn_type,
            reference: r.reference,
            source: r.source,
            status: r.status,
            matched_txn_id: r.matched_txn_id.map(|u| u.to_string()),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

const FEED_COLS: &str =
    "id, organization_id, bank_account_id, txn_date, description, amount, txn_type, \
     reference, source, status, matched_txn_id, created_at, updated_at";

pub struct BankFeedRepo;

impl BankFeedRepo {
    /// Import a batch of feed rows for a bank account. Skips duplicates by (date, amount, description).
    pub async fn import(
        pool: &PgPool,
        org_id: &str,
        bank_account_id: &str,
        input: ImportBankFeed,
    ) -> Result<Vec<BankFeedTransaction>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let acct_uuid = parse_uuid(bank_account_id)?;

        // Verify the bank account belongs to this org.
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM bank_accounts WHERE id = $1 AND organization_id = $2")
                .bind(acct_uuid)
                .bind(org_uuid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;
        if exists.is_none() {
            return Err(DbError::NotFound);
        }

        let mut inserted_ids = Vec::new();

        for row in input.rows {
            let txn_type = if row.amount >= 0 { "credit" } else { "debit" };
            let amount_abs = row.amount.abs();

            // Idempotent: unique index on (bank_account_id, txn_date, amount, description)
            // ensures concurrent imports can't duplicate; ON CONFLICT DO NOTHING is atomic.
            let inserted: Option<(Uuid,)> = sqlx::query_as(
                "INSERT INTO bank_feed_transactions \
                 (organization_id, bank_account_id, txn_date, description, \
                  amount, txn_type, reference, source) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
                 ON CONFLICT (bank_account_id, txn_date, amount, description) DO NOTHING \
                 RETURNING id",
            )
            .bind(org_uuid)
            .bind(acct_uuid)
            .bind(row.txn_date)
            .bind(&row.description)
            .bind(amount_abs)
            .bind(txn_type)
            .bind(&row.reference)
            .bind(&row.source)
            .fetch_optional(pool)
            .await
            .map_err(map_sqlx_err)?;

            if let Some((id,)) = inserted {
                inserted_ids.push(id);
            }
        }

        if inserted_ids.is_empty() {
            return Ok(vec![]);
        }

        let rows: Vec<FeedRow> = sqlx::query_as(&format!(
            "SELECT {FEED_COLS} FROM bank_feed_transactions \
             WHERE id = ANY($1) ORDER BY txn_date ASC"
        ))
        .bind(&inserted_ids)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows.into_iter().map(BankFeedTransaction::from).collect())
    }

    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        bank_account_id: &str,
        status: Option<&str>,
    ) -> Result<Vec<BankFeedTransaction>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let acct_uuid = parse_uuid(bank_account_id)?;

        let rows: Vec<FeedRow> = sqlx::query_as(&format!(
            "SELECT {FEED_COLS} FROM bank_feed_transactions \
             WHERE organization_id = $1 AND bank_account_id = $2 \
               AND ($3::text IS NULL OR status = $3) \
             ORDER BY txn_date DESC, created_at DESC"
        ))
        .bind(org_uuid)
        .bind(acct_uuid)
        .bind(status)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows.into_iter().map(BankFeedTransaction::from).collect())
    }

    pub async fn match_transaction(
        pool: &PgPool,
        org_id: &str,
        feed_id: &str,
        input: MatchBankFeedTransaction,
    ) -> Result<BankFeedTransaction, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let feed_uuid = parse_uuid(feed_id)?;
        let txn_uuid = parse_uuid(&input.bank_transaction_id)?;

        // Verify the target bank_transaction belongs to this org.
        let txn_exists: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM bank_transactions WHERE id = $1 AND organization_id = $2",
        )
        .bind(txn_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;
        if txn_exists.is_none() {
            return Err(DbError::NotFound);
        }

        let rows_affected = sqlx::query(
            "UPDATE bank_feed_transactions \
             SET status = 'matched', matched_txn_id = $3, updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2 AND status = 'pending'",
        )
        .bind(feed_uuid)
        .bind(org_uuid)
        .bind(txn_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows_affected == 0 {
            let row: Option<(String,)> = sqlx::query_as(
                "SELECT status FROM bank_feed_transactions \
                 WHERE id = $1 AND organization_id = $2",
            )
            .bind(feed_uuid)
            .bind(org_uuid)
            .fetch_optional(pool)
            .await
            .map_err(map_sqlx_err)?;
            return match row {
                None => Err(DbError::NotFound),
                Some((s,)) => Err(DbError::Conflict(format!(
                    "feed transaction cannot be matched from status '{s}'"
                ))),
            };
        }

        let row: FeedRow = sqlx::query_as(&format!(
            "SELECT {FEED_COLS} FROM bank_feed_transactions WHERE id = $1"
        ))
        .bind(feed_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.into())
    }

    pub async fn ignore(
        pool: &PgPool,
        org_id: &str,
        feed_id: &str,
    ) -> Result<BankFeedTransaction, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let feed_uuid = parse_uuid(feed_id)?;

        let rows_affected = sqlx::query(
            "UPDATE bank_feed_transactions \
             SET status = 'ignored', updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2 AND status = 'pending'",
        )
        .bind(feed_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows_affected == 0 {
            let row: Option<(String,)> = sqlx::query_as(
                "SELECT status FROM bank_feed_transactions \
                 WHERE id = $1 AND organization_id = $2",
            )
            .bind(feed_uuid)
            .bind(org_uuid)
            .fetch_optional(pool)
            .await
            .map_err(map_sqlx_err)?;
            return match row {
                None => Err(DbError::NotFound),
                Some((s,)) => Err(DbError::Conflict(format!(
                    "feed transaction cannot be ignored from status '{s}'"
                ))),
            };
        }

        let row: FeedRow = sqlx::query_as(&format!(
            "SELECT {FEED_COLS} FROM bank_feed_transactions WHERE id = $1"
        ))
        .bind(feed_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.into())
    }

    /// Auto-match pending feed transactions against existing bank_transactions
    /// by exact amount + date within ±3 days. Returns counts of matched/unmatched.
    pub async fn auto_match(
        pool: &PgPool,
        org_id: &str,
        bank_account_id: &str,
    ) -> Result<BankFeedAutoMatchResult, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let acct_uuid = parse_uuid(bank_account_id)?;

        // Fetch all pending feed transactions.
        let pending: Vec<FeedRow> = sqlx::query_as(&format!(
            "SELECT {FEED_COLS} FROM bank_feed_transactions \
             WHERE organization_id = $1 AND bank_account_id = $2 AND status = 'pending'"
        ))
        .bind(org_uuid)
        .bind(acct_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let total = pending.len();
        let mut matched = 0usize;

        for feed in &pending {
            // Find a bank_transaction with the same amount and txn_type within ±3 days.
            let candidate: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM bank_transactions \
                 WHERE bank_account_id = $1 \
                   AND organization_id = $2 \
                   AND amount = $3 \
                   AND txn_type = $4 \
                   AND txn_date BETWEEN ($5 - INTERVAL '3 days') AND ($5 + INTERVAL '3 days') \
                   AND id NOT IN (
                       SELECT matched_txn_id FROM bank_feed_transactions
                       WHERE matched_txn_id IS NOT NULL
                   )
                 LIMIT 1",
            )
            .bind(acct_uuid)
            .bind(org_uuid)
            .bind(feed.amount)
            .bind(&feed.txn_type)
            .bind(feed.txn_date)
            .fetch_optional(pool)
            .await
            .map_err(map_sqlx_err)?;

            if let Some((txn_id,)) = candidate {
                sqlx::query(
                    "UPDATE bank_feed_transactions \
                     SET status = 'matched', matched_txn_id = $3, updated_at = NOW() \
                     WHERE id = $1 AND organization_id = $2",
                )
                .bind(feed.id)
                .bind(org_uuid)
                .bind(txn_id)
                .execute(pool)
                .await
                .map_err(map_sqlx_err)?;

                matched += 1;
            }
        }

        Ok(BankFeedAutoMatchResult {
            matched,
            unmatched: total - matched,
        })
    }
}
