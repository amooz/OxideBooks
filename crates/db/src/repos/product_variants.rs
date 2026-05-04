use oxidebooks_core::models::{CreateProductVariant, ProductVariant, UpdateProductVariant};
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct VariantRow {
    id: Uuid,
    product_id: Uuid,
    organization_id: Uuid,
    sku: Option<String>,
    name: String,
    attributes: JsonValue,
    price_override: Option<i64>,
    is_active: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<VariantRow> for ProductVariant {
    fn from(r: VariantRow) -> Self {
        ProductVariant {
            id: r.id.to_string(),
            product_id: r.product_id.to_string(),
            organization_id: r.organization_id.to_string(),
            sku: r.sku,
            name: r.name,
            attributes: r.attributes,
            price_override: r.price_override,
            is_active: r.is_active,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

const COLS: &str = "id, product_id, organization_id, sku, name, attributes, \
                    price_override, is_active, created_at, updated_at";

pub struct ProductVariantRepo;

impl ProductVariantRepo {
    pub async fn list_for_product(
        pool: &PgPool,
        org_id: &str,
        product_id: &str,
        active_only: bool,
    ) -> Result<Vec<ProductVariant>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let prod_uuid = parse_uuid(product_id)?;

        // Verify product belongs to org
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM products WHERE organization_id = $1 AND id = $2")
                .bind(org_uuid)
                .bind(prod_uuid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;
        if exists.is_none() {
            return Err(DbError::NotFound);
        }

        let rows: Vec<VariantRow> = if active_only {
            sqlx::query_as(&format!(
                "SELECT {COLS} FROM product_variants \
                 WHERE organization_id = $1 AND product_id = $2 AND is_active = TRUE \
                 ORDER BY name ASC"
            ))
            .bind(org_uuid)
            .bind(prod_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(&format!(
                "SELECT {COLS} FROM product_variants \
                 WHERE organization_id = $1 AND product_id = $2 \
                 ORDER BY name ASC"
            ))
            .bind(org_uuid)
            .bind(prod_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        };

        Ok(rows.into_iter().map(ProductVariant::from).collect())
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        product_id: &str,
        variant_id: &str,
    ) -> Result<ProductVariant, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let prod_uuid = parse_uuid(product_id)?;
        let var_uuid = parse_uuid(variant_id)?;

        let row: VariantRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM product_variants \
             WHERE organization_id = $1 AND product_id = $2 AND id = $3"
        ))
        .bind(org_uuid)
        .bind(prod_uuid)
        .bind(var_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        Ok(row.into())
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        product_id: &str,
        input: CreateProductVariant,
    ) -> Result<ProductVariant, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let prod_uuid = parse_uuid(product_id)?;

        // Verify product belongs to org
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM products WHERE organization_id = $1 AND id = $2")
                .bind(org_uuid)
                .bind(prod_uuid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;
        if exists.is_none() {
            return Err(DbError::NotFound);
        }

        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO product_variants \
             (id, product_id, organization_id, sku, name, attributes, price_override) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(prod_uuid)
        .bind(org_uuid)
        .bind(&input.sku)
        .bind(&input.name)
        .bind(&input.attributes)
        .bind(input.price_override)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, product_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        product_id: &str,
        variant_id: &str,
        input: UpdateProductVariant,
    ) -> Result<ProductVariant, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let prod_uuid = parse_uuid(product_id)?;
        let var_uuid = parse_uuid(variant_id)?;

        let rows = sqlx::query(
            "UPDATE product_variants SET \
             sku            = COALESCE($4, sku), \
             name           = COALESCE($5, name), \
             attributes     = COALESCE($6, attributes), \
             price_override = COALESCE($7, price_override), \
             is_active      = COALESCE($8, is_active), \
             updated_at     = NOW() \
             WHERE organization_id = $1 AND product_id = $2 AND id = $3",
        )
        .bind(org_uuid)
        .bind(prod_uuid)
        .bind(var_uuid)
        .bind(&input.sku)
        .bind(&input.name)
        .bind(&input.attributes)
        .bind(input.price_override)
        .bind(input.is_active)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows == 0 {
            return Err(DbError::NotFound);
        }
        Self::get_by_id(pool, org_id, product_id, variant_id).await
    }

    pub async fn delete(
        pool: &PgPool,
        org_id: &str,
        product_id: &str,
        variant_id: &str,
    ) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let prod_uuid = parse_uuid(product_id)?;
        let var_uuid = parse_uuid(variant_id)?;

        let rows = sqlx::query(
            "DELETE FROM product_variants \
             WHERE organization_id = $1 AND product_id = $2 AND id = $3",
        )
        .bind(org_uuid)
        .bind(prod_uuid)
        .bind(var_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
