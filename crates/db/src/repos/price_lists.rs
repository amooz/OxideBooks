use oxidebooks_core::models::{
    CreatePriceList, PriceList, PriceListItem, SpendAnalysisReport, SpendAnalysisRow,
    UpsertPriceListItem,
};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct PriceListRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    currency: String,
    is_default: bool,
    created_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct ItemRow {
    id: Uuid,
    price_list_id: Uuid,
    product_id: Uuid,
    unit_price: i64,
}

fn list_from_row(r: PriceListRow) -> PriceList {
    PriceList {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        name: r.name,
        currency: r.currency,
        is_default: r.is_default,
        created_at: r.created_at,
    }
}

fn item_from_row(r: ItemRow) -> PriceListItem {
    PriceListItem {
        id: r.id.to_string(),
        price_list_id: r.price_list_id.to_string(),
        product_id: r.product_id.to_string(),
        unit_price: r.unit_price,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

pub struct PriceListRepo;

impl PriceListRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<PriceList>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<PriceListRow> = sqlx::query_as(
            "SELECT id, organization_id, name, currency, is_default, created_at \
             FROM price_lists WHERE organization_id = $1 ORDER BY is_default DESC, name",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(list_from_row).collect())
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreatePriceList,
    ) -> Result<PriceList, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        if input.is_default {
            sqlx::query("UPDATE price_lists SET is_default = false WHERE organization_id = $1")
                .bind(org_uuid)
                .execute(pool)
                .await
                .map_err(map_sqlx_err)?;
        }
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO price_lists (organization_id, name, currency, is_default) \
             VALUES ($1,$2,$3,$4) RETURNING id",
        )
        .bind(org_uuid)
        .bind(&input.name)
        .bind(&input.currency)
        .bind(input.is_default)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: PriceListRow = sqlx::query_as(
            "SELECT id, organization_id, name, currency, is_default, created_at \
             FROM price_lists WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(list_from_row(row))
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n = sqlx::query("DELETE FROM price_lists WHERE id = $1 AND organization_id = $2")
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

    pub async fn list_items(
        pool: &PgPool,
        org_id: &str,
        price_list_id: &str,
    ) -> Result<Vec<PriceListItem>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let pl_uuid = parse_uuid(price_list_id)?;
        let rows: Vec<ItemRow> = sqlx::query_as(
            "SELECT pli.id, pli.price_list_id, pli.product_id, pli.unit_price \
             FROM price_list_items pli \
             JOIN price_lists pl ON pl.id = pli.price_list_id \
             WHERE pli.price_list_id = $1 AND pl.organization_id = $2",
        )
        .bind(pl_uuid)
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(item_from_row).collect())
    }

    pub async fn upsert_item(
        pool: &PgPool,
        org_id: &str,
        price_list_id: &str,
        input: UpsertPriceListItem,
    ) -> Result<PriceListItem, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let pl_uuid = parse_uuid(price_list_id)?;
        let product_uuid = parse_uuid(&input.product_id)?;

        // Verify price list belongs to org
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM price_lists WHERE id = $1 AND organization_id = $2)",
        )
        .bind(pl_uuid)
        .bind(org_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        if !exists {
            return Err(DbError::NotFound);
        }

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO price_list_items (price_list_id, product_id, unit_price) \
             VALUES ($1,$2,$3) \
             ON CONFLICT (price_list_id, product_id) DO UPDATE SET unit_price = EXCLUDED.unit_price \
             RETURNING id",
        )
        .bind(pl_uuid)
        .bind(product_uuid)
        .bind(input.unit_price)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: ItemRow = sqlx::query_as(
            "SELECT id, price_list_id, product_id, unit_price FROM price_list_items WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(item_from_row(row))
    }

    pub async fn spend_analysis(
        pool: &PgPool,
        org_id: &str,
        from: Date,
        to: Date,
    ) -> Result<SpendAnalysisReport, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        #[derive(sqlx::FromRow)]
        struct Row {
            category: String,
            month: String,
            total: i64,
        }

        let rows: Vec<Row> = sqlx::query_as(
            "SELECT \
               category, \
               TO_CHAR(DATE_TRUNC('month', expense_date), 'YYYY-MM') AS month, \
               SUM(amount)::BIGINT AS total \
             FROM expenses \
             WHERE organization_id = $1 \
               AND expense_date BETWEEN $2 AND $3 \
               AND status != 'rejected' \
             GROUP BY category, DATE_TRUNC('month', expense_date) \
             ORDER BY month, total DESC",
        )
        .bind(org_uuid)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let total: i64 = rows.iter().map(|r| r.total).sum();
        let spend_rows = rows
            .into_iter()
            .map(|r| SpendAnalysisRow {
                category: r.category,
                month: r.month,
                total: r.total,
            })
            .collect();

        Ok(SpendAnalysisReport {
            rows: spend_rows,
            total,
        })
    }
}
