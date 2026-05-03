use oxidebooks_core::models::{BankRule, CreateBankRule};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct BankRuleRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    match_field: String,
    match_type: String,
    match_value: String,
    account_id: Uuid,
    auto_description: Option<String>,
    priority: i32,
    created_at: OffsetDateTime,
}

fn from_row(r: BankRuleRow) -> BankRule {
    BankRule {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        name: r.name,
        match_field: r.match_field,
        match_type: r.match_type,
        match_value: r.match_value,
        account_id: r.account_id.to_string(),
        auto_description: r.auto_description,
        priority: r.priority,
        created_at: r.created_at,
    }
}

pub struct BankRuleRepo;

impl BankRuleRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<BankRule>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<BankRuleRow> = sqlx::query_as(
            "SELECT id, organization_id, name, match_field, match_type, match_value, \
             account_id, auto_description, priority, created_at \
             FROM bank_rules WHERE organization_id = $1 ORDER BY priority ASC, created_at ASC",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateBankRule,
    ) -> Result<BankRule, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let account_uuid = parse_uuid(&input.account_id)?;

        let row: BankRuleRow = sqlx::query_as(
            "INSERT INTO bank_rules \
             (organization_id, name, match_field, match_type, match_value, account_id, auto_description, priority) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) \
             RETURNING id, organization_id, name, match_field, match_type, match_value, \
                       account_id, auto_description, priority, created_at",
        )
        .bind(org_uuid)
        .bind(&input.name)
        .bind(&input.match_field)
        .bind(&input.match_type)
        .bind(&input.match_value)
        .bind(account_uuid)
        .bind(&input.auto_description)
        .bind(input.priority)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(from_row(row))
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n = sqlx::query("DELETE FROM bank_rules WHERE id = $1 AND organization_id = $2")
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

    /// Apply all rules (in priority order) to unmatched bank transactions for the given account.
    /// Returns the number of transactions matched.
    pub async fn apply_rules(
        pool: &PgPool,
        org_id: &str,
        bank_account_id: &str,
    ) -> Result<(u64, u64), DbError> {
        let rules = Self::list(pool, org_id).await?;
        let account_uuid = parse_uuid(bank_account_id)?;

        // Fetch unmatched transactions
        #[derive(sqlx::FromRow)]
        struct TxRow {
            id: Uuid,
            description: Option<String>,
            amount: i64,
        }

        let txns: Vec<TxRow> = sqlx::query_as(
            "SELECT id, description, amount FROM bank_transactions \
             WHERE bank_account_id = $1 AND matched = FALSE AND excluded = FALSE",
        )
        .bind(account_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let total = txns.len() as u64;
        let mut matched = 0u64;

        for tx in &txns {
            for rule in &rules {
                let field_val = match rule.match_field.as_str() {
                    "amount" => tx.amount.to_string(),
                    _ => tx.description.clone().unwrap_or_default(),
                };
                let hits = match rule.match_type.as_str() {
                    "contains" => field_val
                        .to_lowercase()
                        .contains(&rule.match_value.to_lowercase()),
                    "equals" => field_val.eq_ignore_ascii_case(&rule.match_value),
                    "gt" => rule
                        .match_value
                        .parse::<i64>()
                        .map(|v| tx.amount > v)
                        .unwrap_or(false),
                    "lt" => rule
                        .match_value
                        .parse::<i64>()
                        .map(|v| tx.amount < v)
                        .unwrap_or(false),
                    _ => false,
                };
                if hits {
                    let account_uuid = parse_uuid(&rule.account_id)?;
                    let _ = sqlx::query(
                        "UPDATE bank_transactions SET matched = TRUE, account_id = $1, \
                         description = COALESCE($2, description) WHERE id = $3",
                    )
                    .bind(account_uuid)
                    .bind(&rule.auto_description)
                    .bind(tx.id)
                    .execute(pool)
                    .await
                    .map_err(map_sqlx_err)?;
                    matched += 1;
                    break;
                }
            }
        }

        Ok((matched, total - matched))
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
