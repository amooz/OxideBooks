use oxidebooks_core::models::{
    AssetRegisterRow, BulkDepreciationResult, CreateFixedAsset, DepreciationMethod,
    DepreciationScheduleLine, FixedAsset, UpdateFixedAsset,
};
use sqlx::PgPool;
use std::str::FromStr;
use time::{Date, Month, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct AssetRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    asset_number: String,
    purchase_date: Date,
    purchase_cost: i64,
    salvage_value: i64,
    useful_life_months: i32,
    depreciation_method: String,
    asset_account_id: Option<Uuid>,
    accumulated_depreciation_acct: Option<Uuid>,
    depreciation_expense_acct: Option<Uuid>,
    status: String,
    disposed_at: Option<Date>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct RegisterRow {
    id: Uuid,
    name: String,
    asset_number: String,
    purchase_cost: i64,
    salvage_value: i64,
    total_depreciated: i64,
    book_value: i64,
    status: String,
}

fn asset_from_row(r: AssetRow, total_depreciated: i64) -> FixedAsset {
    let book_value = (r.purchase_cost - r.salvage_value - total_depreciated).max(0);
    FixedAsset {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        name: r.name,
        asset_number: r.asset_number,
        purchase_date: r.purchase_date,
        purchase_cost: r.purchase_cost,
        salvage_value: r.salvage_value,
        useful_life_months: r.useful_life_months,
        depreciation_method: DepreciationMethod::from_str(&r.depreciation_method)
            .unwrap_or(DepreciationMethod::StraightLine),
        asset_account_id: r.asset_account_id.map(|u| u.to_string()),
        accumulated_depreciation_acct: r.accumulated_depreciation_acct.map(|u| u.to_string()),
        depreciation_expense_acct: r.depreciation_expense_acct.map(|u| u.to_string()),
        status: r.status,
        disposed_at: r.disposed_at,
        total_depreciated,
        book_value,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

const COLS: &str = "id, organization_id, name, asset_number, purchase_date, purchase_cost, \
                    salvage_value, useful_life_months, depreciation_method, asset_account_id, \
                    accumulated_depreciation_acct, depreciation_expense_acct, status, \
                    disposed_at, created_at, updated_at";

async fn get_total_depreciated(pool: &PgPool, asset_id: Uuid) -> Result<i64, DbError> {
    let v: Option<i64> = sqlx::query_scalar(
        "SELECT SUM(amount) FROM asset_depreciation_entries WHERE asset_id = $1",
    )
    .bind(asset_id)
    .fetch_one(pool)
    .await
    .map_err(map_sqlx_err)?;
    Ok(v.unwrap_or(0))
}

pub struct FixedAssetRepo;

impl FixedAssetRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<FixedAsset>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<AssetRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM fixed_assets WHERE organization_id = $1 ORDER BY asset_number"
        ))
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut result = Vec::with_capacity(rows.len());
        for r in rows {
            let td = get_total_depreciated(pool, r.id).await?;
            result.push(asset_from_row(r, td));
        }
        Ok(result)
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<FixedAsset, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: AssetRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM fixed_assets WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        let td = get_total_depreciated(pool, row.id).await?;
        Ok(asset_from_row(row, td))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateFixedAsset,
    ) -> Result<FixedAsset, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let asset_acct = input
            .asset_account_id
            .as_deref()
            .map(parse_uuid)
            .transpose()?;
        let accum_acct = input
            .accumulated_depreciation_acct
            .as_deref()
            .map(parse_uuid)
            .transpose()?;
        let exp_acct = input
            .depreciation_expense_acct
            .as_deref()
            .map(parse_uuid)
            .transpose()?;

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO fixed_assets \
             (organization_id, name, asset_number, purchase_date, purchase_cost, salvage_value, \
              useful_life_months, depreciation_method, asset_account_id, \
              accumulated_depreciation_acct, depreciation_expense_acct) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING id",
        )
        .bind(org_uuid)
        .bind(&input.name)
        .bind(&input.asset_number)
        .bind(input.purchase_date)
        .bind(input.purchase_cost)
        .bind(input.salvage_value)
        .bind(input.useful_life_months)
        .bind(input.depreciation_method.to_string())
        .bind(asset_acct)
        .bind(accum_acct)
        .bind(exp_acct)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateFixedAsset,
    ) -> Result<FixedAsset, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let asset_acct = input
            .asset_account_id
            .as_deref()
            .map(parse_uuid)
            .transpose()?;
        let accum_acct = input
            .accumulated_depreciation_acct
            .as_deref()
            .map(parse_uuid)
            .transpose()?;
        let exp_acct = input
            .depreciation_expense_acct
            .as_deref()
            .map(parse_uuid)
            .transpose()?;

        let n = sqlx::query(
            "UPDATE fixed_assets SET \
             name                            = COALESCE($1, name), \
             asset_account_id                = COALESCE($2, asset_account_id), \
             accumulated_depreciation_acct   = COALESCE($3, accumulated_depreciation_acct), \
             depreciation_expense_acct       = COALESCE($4, depreciation_expense_acct), \
             updated_at                      = NOW() \
             WHERE id = $5 AND organization_id = $6",
        )
        .bind(input.name)
        .bind(asset_acct)
        .bind(accum_acct)
        .bind(exp_acct)
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

    /// Record a monthly depreciation entry.
    pub async fn depreciate(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        period_date: Date,
    ) -> Result<FixedAsset, DbError> {
        let asset = Self::get_by_id(pool, org_id, id).await?;
        if asset.status != "active" {
            return Err(DbError::Conflict("asset is not active".into()));
        }

        let n = asset.useful_life_months as i64;
        let depreciable = asset.purchase_cost - asset.salvage_value;

        // For SYD we need to know which period number we're on.
        let period_num = if asset.depreciation_method == DepreciationMethod::SumOfYearsDigits {
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM asset_depreciation_entries WHERE asset_id = $1",
            )
            .bind(parse_uuid(id)?)
            .fetch_one(pool)
            .await
            .map_err(map_sqlx_err)?;
            count + 1
        } else {
            0
        };

        let amount = match asset.depreciation_method {
            DepreciationMethod::StraightLine => depreciable / n,
            DepreciationMethod::DecliningBalance => {
                let book = asset.book_value;
                (book * 2 / n).max(0)
            }
            DepreciationMethod::SumOfYearsDigits => {
                let syd = n * (n + 1) / 2;
                let remaining_periods = (n - period_num + 1).max(0);
                if syd == 0 {
                    0
                } else {
                    depreciable * remaining_periods / syd
                }
            }
        };

        let remaining = depreciable - asset.total_depreciated;
        let amount = amount.min(remaining).max(0);

        if amount == 0 {
            return Ok(asset);
        }

        let asset_uuid = parse_uuid(id)?;

        sqlx::query(
            "INSERT INTO asset_depreciation_entries (asset_id, period_date, amount) \
             VALUES ($1, $2, $3) ON CONFLICT (asset_id, period_date) DO NOTHING",
        )
        .bind(asset_uuid)
        .bind(period_date)
        .bind(amount)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        sqlx::query("UPDATE fixed_assets SET updated_at = NOW() WHERE id = $1")
            .bind(asset_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, id).await
    }

    /// Dispose (retire) a fixed asset.
    pub async fn dispose(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        disposal_date: Date,
    ) -> Result<FixedAsset, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let n = sqlx::query(
            "UPDATE fixed_assets SET status = 'disposed', disposed_at = $1, updated_at = NOW() \
             WHERE id = $2 AND organization_id = $3 AND status = 'active'",
        )
        .bind(disposal_date)
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

    /// Generate the complete depreciation schedule (posted + projected future periods).
    pub async fn schedule(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<Vec<DepreciationScheduleLine>, DbError> {
        let asset = Self::get_by_id(pool, org_id, id).await?;
        let asset_uuid = parse_uuid(id)?;

        // Fetch posted entries keyed by date.
        #[derive(sqlx::FromRow)]
        struct EntryRow {
            period_date: Date,
            amount: i64,
        }
        let posted: Vec<EntryRow> = sqlx::query_as(
            "SELECT period_date, amount FROM asset_depreciation_entries \
             WHERE asset_id = $1 ORDER BY period_date",
        )
        .bind(asset_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let posted_map: std::collections::HashMap<Date, i64> =
            posted.iter().map(|e| (e.period_date, e.amount)).collect();

        let n = asset.useful_life_months as i64;
        let depreciable = asset.purchase_cost - asset.salvage_value;
        let syd = n * (n + 1) / 2;
        let start = first_of_month(asset.purchase_date);

        let mut lines = Vec::with_capacity(asset.useful_life_months as usize);
        let mut accumulated: i64 = 0;

        for period in 1..=asset.useful_life_months as i64 {
            let date = add_months(start, (period - 1) as i32);
            let is_posted = posted_map.contains_key(&date);

            let amount = if is_posted {
                *posted_map.get(&date).unwrap()
            } else {
                // Project: compute what the amount would be.
                let projected = match asset.depreciation_method {
                    DepreciationMethod::StraightLine => depreciable / n,
                    DepreciationMethod::DecliningBalance => {
                        let book = (depreciable - accumulated).max(0);
                        (book * 2 / n).max(0)
                    }
                    DepreciationMethod::SumOfYearsDigits => {
                        let remaining = (n - period + 1).max(0);
                        if syd == 0 {
                            0
                        } else {
                            depreciable * remaining / syd
                        }
                    }
                };
                let remaining_depreciable = (depreciable - accumulated).max(0);
                projected.min(remaining_depreciable).max(0)
            };

            accumulated += amount;
            let book_value = (asset.purchase_cost - accumulated).max(asset.salvage_value);

            lines.push(DepreciationScheduleLine {
                period: period as i32,
                period_date: date,
                amount,
                accumulated_depreciation: accumulated,
                book_value,
                is_posted,
            });

            if accumulated >= depreciable {
                break;
            }
        }

        Ok(lines)
    }

    /// Depreciate all active assets for a given period. Skips already-posted periods.
    pub async fn bulk_depreciate(
        pool: &PgPool,
        org_id: &str,
        period_date: Date,
    ) -> Result<BulkDepreciationResult, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let asset_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM fixed_assets WHERE organization_id = $1 AND status = 'active'",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let processed = asset_ids.len();
        let mut succeeded = 0usize;
        let mut skipped = 0usize;

        for asset_id in asset_ids {
            let id_str = asset_id.to_string();
            let asset = Self::get_by_id(pool, org_id, &id_str).await?;

            // Skip if already fully depreciated.
            if asset.book_value <= asset.salvage_value {
                skipped += 1;
                continue;
            }

            // Check if this period is already posted.
            let already: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM asset_depreciation_entries \
                 WHERE asset_id = $1 AND period_date = $2)",
            )
            .bind(asset_id)
            .bind(period_date)
            .fetch_one(pool)
            .await
            .map_err(map_sqlx_err)?;

            if already {
                skipped += 1;
                continue;
            }

            match Self::depreciate(pool, org_id, &id_str, period_date).await {
                Ok(_) => succeeded += 1,
                Err(_) => skipped += 1,
            }
        }

        Ok(BulkDepreciationResult {
            period_date: period_date.to_string(),
            processed,
            succeeded,
            skipped,
        })
    }

    pub async fn asset_register(
        pool: &PgPool,
        org_id: &str,
    ) -> Result<Vec<AssetRegisterRow>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<RegisterRow> = sqlx::query_as(
            "SELECT a.id, a.name, a.asset_number, a.purchase_cost, a.salvage_value, \
             COALESCE(d.total, 0) AS total_depreciated, \
             GREATEST(a.purchase_cost - a.salvage_value - COALESCE(d.total, 0), 0) AS book_value, \
             a.status \
             FROM fixed_assets a \
             LEFT JOIN (SELECT asset_id, SUM(amount) AS total FROM asset_depreciation_entries \
                        GROUP BY asset_id) d ON d.asset_id = a.id \
             WHERE a.organization_id = $1 \
             ORDER BY a.asset_number",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows
            .into_iter()
            .map(|r| AssetRegisterRow {
                id: r.id.to_string(),
                name: r.name,
                asset_number: r.asset_number,
                purchase_cost: r.purchase_cost,
                salvage_value: r.salvage_value,
                total_depreciated: r.total_depreciated,
                book_value: r.book_value,
                status: r.status,
            })
            .collect())
    }
}

fn first_of_month(date: Date) -> Date {
    Date::from_calendar_date(date.year(), date.month(), 1).expect("valid date")
}

fn add_months(date: Date, months: i32) -> Date {
    let total = date.month() as i32 - 1 + months;
    let year = date.year() + total.div_euclid(12);
    let month = Month::try_from((total.rem_euclid(12) + 1) as u8).expect("valid month");
    Date::from_calendar_date(year, month, 1).expect("valid date")
}
