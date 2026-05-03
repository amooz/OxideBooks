use oxidebooks_core::models::{CreateLandedCost, LandedCost, LandedCostAllocation};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct LcRow {
    id: Uuid,
    organization_id: Uuid,
    grn_id: Uuid,
    description: String,
    amount: i64,
    allocation_method: String,
    currency: String,
    vendor_id: Option<Uuid>,
    created_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct AllocRow {
    id: Uuid,
    landed_cost_id: Uuid,
    grn_line_id: Uuid,
    allocated_amount: i64,
}

async fn load_allocations(
    pool: &PgPool,
    lc_id: Uuid,
) -> Result<Vec<LandedCostAllocation>, DbError> {
    let rows: Vec<AllocRow> = sqlx::query_as(
        "SELECT id, landed_cost_id, grn_line_id, allocated_amount
         FROM landed_cost_allocations WHERE landed_cost_id = $1",
    )
    .bind(lc_id)
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_err)?;
    Ok(rows
        .into_iter()
        .map(|r| LandedCostAllocation {
            id: r.id.to_string(),
            landed_cost_id: r.landed_cost_id.to_string(),
            grn_line_id: r.grn_line_id.to_string(),
            allocated_amount: r.allocated_amount,
        })
        .collect())
}

fn lc_from_row(r: LcRow, allocations: Vec<LandedCostAllocation>) -> LandedCost {
    LandedCost {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        grn_id: r.grn_id.to_string(),
        description: r.description,
        amount: r.amount,
        allocation_method: r.allocation_method,
        currency: r.currency,
        vendor_id: r.vendor_id.map(|u| u.to_string()),
        allocations,
        created_at: r.created_at,
    }
}

pub struct LandedCostRepo;

impl LandedCostRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        grn_id: &str,
    ) -> Result<Vec<LandedCost>, DbError> {
        let org = parse_uuid(org_id)?;
        let gid = parse_uuid(grn_id)?;
        // verify GRN belongs to org
        let exists: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM goods_receipt_notes WHERE organization_id = $1 AND id = $2",
        )
        .bind(org)
        .bind(gid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;
        if exists.is_none() {
            return Err(DbError::NotFound);
        }
        let rows: Vec<LcRow> = sqlx::query_as(
            "SELECT id, organization_id, grn_id, description, amount, allocation_method,
                    currency, vendor_id, created_at
             FROM landed_costs WHERE grn_id = $1 ORDER BY created_at",
        )
        .bind(gid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            let allocs = load_allocations(pool, row.id).await?;
            result.push(lc_from_row(row, allocs));
        }
        Ok(result)
    }

    /// Create a landed cost and proportionally allocate it across GRN lines.
    /// For 'quantity' method: each line's share = line.quantity_received / total_qty
    /// For 'value' method: each line's share = line.unit_cost * line.qty / total_value
    /// Also updates inventory_items.cost_per_unit for each affected item.
    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        grn_id: &str,
        input: CreateLandedCost,
    ) -> Result<LandedCost, DbError> {
        let org = parse_uuid(org_id)?;
        let gid = parse_uuid(grn_id)?;

        // Fetch GRN and verify ownership + posted status
        let grn_status: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT id, status FROM goods_receipt_notes WHERE organization_id = $1 AND id = $2",
        )
        .bind(org)
        .bind(gid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;
        let (_, status) = grn_status.ok_or(DbError::NotFound)?;
        if status != "posted" {
            return Err(DbError::Conflict(
                "landed costs can only be added to posted GRNs".into(),
            ));
        }

        // Fetch GRN lines with item info
        let lines: Vec<(Uuid, Uuid, Option<Uuid>, i64, i64)> = sqlx::query_as(
            "SELECT id, po_line_id, item_id, quantity_received, unit_cost
             FROM grn_lines WHERE grn_id = $1",
        )
        .bind(gid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        if lines.is_empty() {
            return Err(DbError::Conflict("GRN has no lines to allocate to".into()));
        }

        let vendor_id = input.vendor_id.as_deref().map(parse_uuid).transpose()?;

        let lc_id: Uuid = sqlx::query_scalar(
            "INSERT INTO landed_costs
                (organization_id, grn_id, description, amount, allocation_method, currency, vendor_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING id",
        )
        .bind(org)
        .bind(gid)
        .bind(&input.description)
        .bind(input.amount)
        .bind(&input.allocation_method)
        .bind(&input.currency)
        .bind(vendor_id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        // Compute allocation weights
        let total_weight: i64 = if input.allocation_method == "value" {
            lines.iter().map(|l| l.3 * l.4).sum()
        } else {
            lines.iter().map(|l| l.3).sum()
        };

        if total_weight == 0 {
            return Err(DbError::Conflict(
                "cannot allocate: total weight is zero".into(),
            ));
        }

        let mut remaining = input.amount;
        let n = lines.len();
        for (i, line) in lines.iter().enumerate() {
            let (line_id, _po_line_id, item_id, qty, unit_cost) = line;
            let weight = if input.allocation_method == "value" {
                qty * unit_cost
            } else {
                *qty
            };
            // Give the last line the remainder to avoid rounding drift
            let alloc = if i == n - 1 {
                remaining
            } else {
                (input.amount as i128 * weight as i128 / total_weight as i128) as i64
            };
            remaining -= alloc;

            sqlx::query(
                "INSERT INTO landed_cost_allocations (landed_cost_id, grn_line_id, allocated_amount)
                 VALUES ($1, $2, $3)",
            )
            .bind(lc_id)
            .bind(line_id)
            .bind(alloc)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;

            // Adjust inventory_items.cost_per_unit if item is tracked
            if let Some(iid) = item_id {
                // Add alloc / qty to cost_per_unit (integer cents per unit)
                let extra_per_unit = if *qty > 0 { alloc / qty } else { 0 };
                if extra_per_unit > 0 {
                    sqlx::query(
                        "UPDATE inventory_items
                         SET cost_per_unit = cost_per_unit + $1
                         WHERE id = $2",
                    )
                    .bind(extra_per_unit)
                    .bind(iid)
                    .execute(pool)
                    .await
                    .map_err(map_sqlx_err)?;
                }
            }
        }

        // Fetch and return the created landed cost
        let row: LcRow = sqlx::query_as(
            "SELECT id, organization_id, grn_id, description, amount, allocation_method,
                    currency, vendor_id, created_at
             FROM landed_costs WHERE id = $1",
        )
        .bind(lc_id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        let allocs = load_allocations(pool, lc_id).await?;
        Ok(lc_from_row(row, allocs))
    }
}
