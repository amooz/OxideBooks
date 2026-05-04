use oxidebooks_core::models::{CreateTaxRule, SuggestedTaxRate, TaxRule, UpdateTaxRule};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

const COLS: &str =
    "id, organization_id, name, country_code, region_code, tax_rate_id, applies_to, \
     is_active, priority, created_at, updated_at";

#[derive(sqlx::FromRow)]
struct TaxRuleRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    country_code: String,
    region_code: Option<String>,
    tax_rate_id: Uuid,
    applies_to: String,
    is_active: bool,
    priority: i32,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn from_row(r: TaxRuleRow) -> TaxRule {
    TaxRule {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        name: r.name,
        country_code: r.country_code,
        region_code: r.region_code,
        tax_rate_id: r.tax_rate_id.to_string(),
        applies_to: r.applies_to,
        is_active: r.is_active,
        priority: r.priority,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

pub struct TaxRuleRepo;

impl TaxRuleRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        applies_to: Option<&str>,
        active_only: bool,
    ) -> Result<Vec<TaxRule>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<TaxRuleRow> = match (applies_to, active_only) {
            (Some(at), true) => sqlx::query_as(&format!(
                "SELECT {COLS} FROM tax_rules WHERE organization_id = $1 \
                 AND applies_to = $2 AND is_active = TRUE \
                 ORDER BY priority DESC, country_code, region_code"
            ))
            .bind(org_uuid)
            .bind(at)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?,
            (Some(at), false) => sqlx::query_as(&format!(
                "SELECT {COLS} FROM tax_rules WHERE organization_id = $1 \
                 AND applies_to = $2 \
                 ORDER BY priority DESC, country_code, region_code"
            ))
            .bind(org_uuid)
            .bind(at)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?,
            (None, true) => sqlx::query_as(&format!(
                "SELECT {COLS} FROM tax_rules WHERE organization_id = $1 AND is_active = TRUE \
                 ORDER BY priority DESC, country_code, region_code"
            ))
            .bind(org_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?,
            (None, false) => sqlx::query_as(&format!(
                "SELECT {COLS} FROM tax_rules WHERE organization_id = $1 \
                 ORDER BY priority DESC, country_code, region_code"
            ))
            .bind(org_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?,
        };
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<TaxRule, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: TaxRuleRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM tax_rules WHERE organization_id = $1 AND id = $2"
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
        input: CreateTaxRule,
    ) -> Result<TaxRule, DbError> {
        validate_applies_to(&input.applies_to)?;
        let org_uuid = parse_uuid(org_id)?;
        let rate_uuid = parse_uuid(&input.tax_rate_id)?;

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO tax_rules \
             (organization_id, name, country_code, region_code, tax_rate_id, applies_to, priority) \
             VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id",
        )
        .bind(org_uuid)
        .bind(&input.name)
        .bind(&input.country_code)
        .bind(&input.region_code)
        .bind(rate_uuid)
        .bind(&input.applies_to)
        .bind(input.priority)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateTaxRule,
    ) -> Result<TaxRule, DbError> {
        if let Some(ref at) = input.applies_to {
            validate_applies_to(at)?;
        }
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rate_uuid = input.tax_rate_id.as_deref().map(parse_uuid).transpose()?;

        sqlx::query(
            "UPDATE tax_rules SET \
             name        = COALESCE($1, name), \
             tax_rate_id = COALESCE($2, tax_rate_id), \
             applies_to  = COALESCE($3, applies_to), \
             is_active   = COALESCE($4, is_active), \
             priority    = COALESCE($5, priority), \
             updated_at  = NOW() \
             WHERE id = $6 AND organization_id = $7",
        )
        .bind(input.name)
        .bind(rate_uuid)
        .bind(input.applies_to)
        .bind(input.is_active)
        .bind(input.priority)
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows = sqlx::query("DELETE FROM tax_rules WHERE id = $1 AND organization_id = $2")
            .bind(id_uuid)
            .bind(org_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?
            .rows_affected();
        if rows == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    /// Return the best-matching tax rate for a given jurisdiction.
    /// Region-specific rules take precedence over country-level rules.
    pub async fn suggest_for_contact(
        pool: &PgPool,
        org_id: &str,
        country_code: &str,
        region_code: Option<&str>,
        applies_to: &str,
    ) -> Result<SuggestedTaxRate, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        #[derive(sqlx::FromRow)]
        struct SuggestRow {
            tax_rate_id: Uuid,
            tax_rate_name: String,
            rate_bps: i32,
            matched_rule_id: Uuid,
        }

        let row: Option<SuggestRow> = sqlx::query_as(
            "SELECT tr.id AS tax_rate_id, tr.name AS tax_rate_name, tr.rate_bps, \
             rule.id AS matched_rule_id \
             FROM tax_rules rule \
             JOIN tax_rates tr ON tr.id = rule.tax_rate_id \
             WHERE rule.organization_id = $1 \
               AND rule.country_code = $2 \
               AND rule.is_active = TRUE \
               AND (rule.applies_to = $3 OR rule.applies_to = 'both') \
               AND (rule.region_code IS NULL \
                    OR ($4::text IS NOT NULL AND rule.region_code = $4)) \
             ORDER BY (rule.region_code IS NOT NULL) DESC, rule.priority DESC \
             LIMIT 1",
        )
        .bind(org_uuid)
        .bind(country_code)
        .bind(applies_to)
        .bind(region_code)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(match row {
            Some(r) => SuggestedTaxRate {
                tax_rate_id: Some(r.tax_rate_id.to_string()),
                tax_rate_name: Some(r.tax_rate_name),
                rate_bps: Some(r.rate_bps),
                matched_rule_id: Some(r.matched_rule_id.to_string()),
            },
            None => SuggestedTaxRate {
                tax_rate_id: None,
                tax_rate_name: None,
                rate_bps: None,
                matched_rule_id: None,
            },
        })
    }
}

fn validate_applies_to(s: &str) -> Result<(), DbError> {
    if matches!(s, "sales" | "purchases" | "both") {
        Ok(())
    } else {
        Err(DbError::Conflict(
            "applies_to must be 'sales', 'purchases', or 'both'".into(),
        ))
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
