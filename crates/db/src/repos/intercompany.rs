use oxidebooks_core::models::{
    CreateIntercompanyLink, CreateIntercompanyTransaction, IntercompanyLink,
    IntercompanyTransaction,
};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::{
    error::{map_sqlx_err, DbError},
    repos::TransactionRepo,
};
use oxidebooks_core::models::{CreateJournalEntry, CreateJournalLine};

#[derive(sqlx::FromRow)]
struct LinkRow {
    id: Uuid,
    organization_id: Uuid,
    counterparty_org_id: Uuid,
    due_from_account_id: Option<Uuid>,
    due_to_account_id: Option<Uuid>,
    created_at: OffsetDateTime,
}

fn link_from_row(r: LinkRow) -> IntercompanyLink {
    IntercompanyLink {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        counterparty_org_id: r.counterparty_org_id.to_string(),
        due_from_account_id: r.due_from_account_id.map(|u| u.to_string()),
        due_to_account_id: r.due_to_account_id.map(|u| u.to_string()),
        created_at: r.created_at,
    }
}

#[derive(sqlx::FromRow)]
struct TxnRow {
    id: Uuid,
    org_a_id: Uuid,
    journal_entry_a: Uuid,
    org_b_id: Uuid,
    journal_entry_b: Uuid,
    amount: i64,
    currency: String,
    description: Option<String>,
    transaction_date: Date,
    created_at: OffsetDateTime,
}

fn txn_from_row(r: TxnRow) -> IntercompanyTransaction {
    IntercompanyTransaction {
        id: r.id.to_string(),
        org_a_id: r.org_a_id.to_string(),
        journal_entry_a: r.journal_entry_a.to_string(),
        org_b_id: r.org_b_id.to_string(),
        journal_entry_b: r.journal_entry_b.to_string(),
        amount: r.amount,
        currency: r.currency,
        description: r.description,
        transaction_date: r.transaction_date,
        created_at: r.created_at,
    }
}

pub struct IntercompanyRepo;

impl IntercompanyRepo {
    // ─── Links ────────────────────────────────────────────────────────────────

    pub async fn list_links(pool: &PgPool, org_id: &str) -> Result<Vec<IntercompanyLink>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<LinkRow> = sqlx::query_as(
            "SELECT id, organization_id, counterparty_org_id, due_from_account_id, \
             due_to_account_id, created_at \
             FROM intercompany_links WHERE organization_id = $1 ORDER BY created_at ASC",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(link_from_row).collect())
    }

    pub async fn create_link(
        pool: &PgPool,
        org_id: &str,
        input: CreateIntercompanyLink,
    ) -> Result<IntercompanyLink, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let cp_uuid = parse_uuid(&input.counterparty_org_id)?;
        let from_uuid = input
            .due_from_account_id
            .as_deref()
            .map(parse_uuid)
            .transpose()?;
        let to_uuid = input
            .due_to_account_id
            .as_deref()
            .map(parse_uuid)
            .transpose()?;

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO intercompany_links \
             (organization_id, counterparty_org_id, due_from_account_id, due_to_account_id) \
             VALUES ($1,$2,$3,$4) RETURNING id",
        )
        .bind(org_uuid)
        .bind(cp_uuid)
        .bind(from_uuid)
        .bind(to_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: LinkRow = sqlx::query_as(
            "SELECT id, organization_id, counterparty_org_id, due_from_account_id, \
             due_to_account_id, created_at \
             FROM intercompany_links WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(link_from_row(row))
    }

    pub async fn delete_link(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows =
            sqlx::query("DELETE FROM intercompany_links WHERE organization_id = $1 AND id = $2")
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

    // ─── Transactions ─────────────────────────────────────────────────────────

    pub async fn list_transactions(
        pool: &PgPool,
        org_id: &str,
    ) -> Result<Vec<IntercompanyTransaction>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<TxnRow> = sqlx::query_as(
            "SELECT id, org_a_id, journal_entry_a, org_b_id, journal_entry_b, \
             amount, currency, description, transaction_date, created_at \
             FROM intercompany_transactions \
             WHERE org_a_id = $1 OR org_b_id = $1 \
             ORDER BY transaction_date DESC, created_at DESC",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(txn_from_row).collect())
    }

    /// Creates symmetric journal entries in both orgs and records the link.
    /// `user_id` is the actor (must have admin rights to both orgs — enforced in the handler).
    pub async fn create_transaction(
        pool: &PgPool,
        org_a_id: &str,
        user_id: &str,
        input: CreateIntercompanyTransaction,
    ) -> Result<IntercompanyTransaction, DbError> {
        if input.amount <= 0 {
            return Err(DbError::Conflict("amount must be positive".into()));
        }
        let description = input.description.clone().unwrap_or_default();

        // Build and post JE for org A.
        let je_a = TransactionRepo::create_posted(
            pool,
            org_a_id,
            user_id,
            CreateJournalEntry {
                date: input.transaction_date,
                description: description.clone(),
                reference: None,
                lines: vec![
                    CreateJournalLine {
                        account_id: input.debit_account_id_a.clone(),
                        description: None,
                        debit: input.amount,
                        credit: 0,
                    },
                    CreateJournalLine {
                        account_id: input.credit_account_id_a.clone(),
                        description: None,
                        debit: 0,
                        credit: input.amount,
                    },
                ],
            },
        )
        .await?;

        // Build and post JE for org B.
        let org_b_id = &input.counterparty_org_id;
        let je_b = TransactionRepo::create_posted(
            pool,
            org_b_id,
            user_id,
            CreateJournalEntry {
                date: input.transaction_date,
                description: description.clone(),
                reference: None,
                lines: vec![
                    CreateJournalLine {
                        account_id: input.debit_account_id_b.clone(),
                        description: None,
                        debit: input.amount,
                        credit: 0,
                    },
                    CreateJournalLine {
                        account_id: input.credit_account_id_b.clone(),
                        description: None,
                        debit: 0,
                        credit: input.amount,
                    },
                ],
            },
        )
        .await?;

        let org_a_uuid = parse_uuid(org_a_id)?;
        let org_b_uuid = parse_uuid(org_b_id)?;
        let je_a_uuid = parse_uuid(&je_a.id)?;
        let je_b_uuid = parse_uuid(&je_b.id)?;

        let row: TxnRow = sqlx::query_as(
            "INSERT INTO intercompany_transactions \
             (org_a_id, journal_entry_a, org_b_id, journal_entry_b, \
              amount, currency, description, transaction_date) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) \
             RETURNING id, org_a_id, journal_entry_a, org_b_id, journal_entry_b, \
                       amount, currency, description, transaction_date, created_at",
        )
        .bind(org_a_uuid)
        .bind(je_a_uuid)
        .bind(org_b_uuid)
        .bind(je_b_uuid)
        .bind(input.amount)
        .bind(&input.currency)
        .bind(&input.description)
        .bind(input.transaction_date)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(txn_from_row(row))
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
