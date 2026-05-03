use oxidebooks_core::models::{CreateProduct, Product, UpdateProduct};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct ProductRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    description: Option<String>,
    sku: Option<String>,
    unit_price: i64,
    currency: String,
    account_id: Option<Uuid>,
    tax_rate_id: Option<Uuid>,
    is_active: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn from_row(r: ProductRow) -> Product {
    Product {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        name: r.name,
        description: r.description,
        sku: r.sku,
        unit_price: r.unit_price,
        currency: r.currency,
        account_id: r.account_id.map(|u| u.to_string()),
        tax_rate_id: r.tax_rate_id.map(|u| u.to_string()),
        is_active: r.is_active,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

const COLS: &str = "id, organization_id, name, description, sku, unit_price, currency, \
                    account_id, tax_rate_id, is_active, created_at, updated_at";

pub struct ProductRepo;

impl ProductRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<Product>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<ProductRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM products WHERE organization_id = $1 ORDER BY name"
        ))
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<Product, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: ProductRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM products WHERE organization_id = $1 AND id = $2"
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
        input: CreateProduct,
    ) -> Result<Product, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let acct_uuid = input.account_id.as_deref().map(parse_uuid).transpose()?;
        let tax_uuid = input.tax_rate_id.as_deref().map(parse_uuid).transpose()?;

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO products \
             (organization_id, name, description, sku, unit_price, currency, account_id, tax_rate_id) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING id",
        )
        .bind(org_uuid)
        .bind(&input.name)
        .bind(&input.description)
        .bind(&input.sku)
        .bind(input.unit_price)
        .bind(&input.currency)
        .bind(acct_uuid)
        .bind(tax_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateProduct,
    ) -> Result<Product, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let acct_uuid = input.account_id.as_deref().map(parse_uuid).transpose()?;
        let tax_uuid = input.tax_rate_id.as_deref().map(parse_uuid).transpose()?;

        let n = sqlx::query(
            "UPDATE products SET \
             name        = COALESCE($1, name), \
             description = COALESCE($2, description), \
             sku         = COALESCE($3, sku), \
             unit_price  = COALESCE($4, unit_price), \
             account_id  = COALESCE($5, account_id), \
             tax_rate_id = COALESCE($6, tax_rate_id), \
             is_active   = COALESCE($7, is_active), \
             updated_at  = NOW() \
             WHERE id = $8 AND organization_id = $9",
        )
        .bind(input.name)
        .bind(input.description)
        .bind(input.sku)
        .bind(input.unit_price)
        .bind(acct_uuid)
        .bind(tax_uuid)
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
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n = sqlx::query("DELETE FROM products WHERE id = $1 AND organization_id = $2")
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
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
