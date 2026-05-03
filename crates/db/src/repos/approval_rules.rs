use oxidebooks_core::models::{ApprovalRule, CreateApprovalRule, UpdateApprovalRule};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct RuleRow {
    id: Uuid,
    organization_id: Uuid,
    entity_type: String,
    name: String,
    min_amount: Option<i64>,
    max_amount: Option<i64>,
    required_role: String,
    is_active: bool,
    sort_order: i32,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn from_row(r: RuleRow) -> ApprovalRule {
    ApprovalRule {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        entity_type: r.entity_type,
        name: r.name,
        min_amount: r.min_amount,
        max_amount: r.max_amount,
        required_role: r.required_role,
        is_active: r.is_active,
        sort_order: r.sort_order,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

const COLS: &str = "id, organization_id, entity_type, name, min_amount, max_amount,
     required_role, is_active, sort_order, created_at, updated_at";

pub struct ApprovalRuleRepo;

impl ApprovalRuleRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        entity_type: Option<&str>,
    ) -> Result<Vec<ApprovalRule>, DbError> {
        let org = parse_uuid(org_id)?;
        let rows = sqlx::query_as::<_, RuleRow>(&format!(
            "SELECT {COLS} FROM approval_rules
             WHERE organization_id = $1
               AND ($2::TEXT IS NULL OR entity_type = $2)
             ORDER BY sort_order, created_at"
        ))
        .bind(org)
        .bind(entity_type)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<ApprovalRule, DbError> {
        let org = parse_uuid(org_id)?;
        let rid = parse_uuid(id)?;
        let row = sqlx::query_as::<_, RuleRow>(&format!(
            "SELECT {COLS} FROM approval_rules WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org)
        .bind(rid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(from_row(row))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateApprovalRule,
    ) -> Result<ApprovalRule, DbError> {
        let org = parse_uuid(org_id)?;
        let valid_types = ["expense", "bill", "purchase_order", "purchase_requisition"];
        if !valid_types.contains(&input.entity_type.as_str()) {
            return Err(DbError::Conflict(format!(
                "invalid entity_type '{}'",
                input.entity_type
            )));
        }
        let valid_roles = ["accountant", "admin", "owner"];
        if !valid_roles.contains(&input.required_role.as_str()) {
            return Err(DbError::Conflict(format!(
                "invalid required_role '{}'",
                input.required_role
            )));
        }
        let row = sqlx::query_as::<_, RuleRow>(&format!(
            "INSERT INTO approval_rules
                (organization_id, entity_type, name, min_amount, max_amount,
                 required_role, sort_order)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING {COLS}"
        ))
        .bind(org)
        .bind(&input.entity_type)
        .bind(&input.name)
        .bind(input.min_amount)
        .bind(input.max_amount)
        .bind(&input.required_role)
        .bind(input.sort_order)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(from_row(row))
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateApprovalRule,
    ) -> Result<ApprovalRule, DbError> {
        let org = parse_uuid(org_id)?;
        let rid = parse_uuid(id)?;
        let row = sqlx::query_as::<_, RuleRow>(&format!(
            "UPDATE approval_rules
             SET name          = COALESCE($3, name),
                 min_amount    = COALESCE($4, min_amount),
                 max_amount    = COALESCE($5, max_amount),
                 required_role = COALESCE($6, required_role),
                 is_active     = COALESCE($7, is_active),
                 sort_order    = COALESCE($8, sort_order),
                 updated_at    = now()
             WHERE organization_id = $1 AND id = $2
             RETURNING {COLS}"
        ))
        .bind(org)
        .bind(rid)
        .bind(&input.name)
        .bind(input.min_amount)
        .bind(input.max_amount)
        .bind(&input.required_role)
        .bind(input.is_active)
        .bind(input.sort_order)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(from_row(row))
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org = parse_uuid(org_id)?;
        let rid = parse_uuid(id)?;
        let n = sqlx::query("DELETE FROM approval_rules WHERE organization_id = $1 AND id = $2")
            .bind(org)
            .bind(rid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?
            .rows_affected();
        if n == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    /// Evaluate rules for a given entity and return the required role,
    /// or None if no rule applies (auto-approve).
    pub async fn required_role_for(
        pool: &PgPool,
        org_id: &str,
        entity_type: &str,
        amount: i64,
    ) -> Result<Option<String>, DbError> {
        let org = parse_uuid(org_id)?;
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT required_role FROM approval_rules
             WHERE organization_id = $1
               AND entity_type = $2
               AND is_active = TRUE
               AND (min_amount IS NULL OR $3 >= min_amount)
               AND (max_amount IS NULL OR $3 <= max_amount)
             ORDER BY sort_order, created_at
             LIMIT 1",
        )
        .bind(org)
        .bind(entity_type)
        .bind(amount)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(row.map(|r| r.0))
    }
}
