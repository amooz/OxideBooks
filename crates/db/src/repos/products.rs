use oxidebooks_core::models::{
    BundleComponent, CreateProduct, Product, SetBundleComponents, UpdateProduct,
};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

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
    category_id: Option<Uuid>,
    is_active: bool,
    is_bundle: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct ComponentRow {
    id: Uuid,
    product_id: Uuid,
    component_id: Uuid,
    component_name: String,
    quantity: i64,
    sort_order: i32,
}

async fn fetch_components(
    pool: &PgPool,
    product_id: Uuid,
) -> Result<Vec<BundleComponent>, DbError> {
    let rows: Vec<ComponentRow> = sqlx::query_as(
        "SELECT pbc.id, pbc.product_id, pbc.component_id, p.name AS component_name,
                pbc.quantity, pbc.sort_order
         FROM product_bundle_components pbc
         JOIN products p ON p.id = pbc.component_id
         WHERE pbc.product_id = $1 ORDER BY pbc.sort_order, pbc.id",
    )
    .bind(product_id)
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_err)?;
    Ok(rows
        .into_iter()
        .map(|r| BundleComponent {
            id: r.id.to_string(),
            product_id: r.product_id.to_string(),
            component_id: r.component_id.to_string(),
            component_name: r.component_name,
            quantity: r.quantity,
            sort_order: r.sort_order,
        })
        .collect())
}

async fn product_from_row(pool: &PgPool, r: ProductRow) -> Result<Product, DbError> {
    let bundle_components = if r.is_bundle {
        fetch_components(pool, r.id).await?
    } else {
        Vec::new()
    };
    Ok(Product {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        name: r.name,
        description: r.description,
        sku: r.sku,
        unit_price: r.unit_price,
        currency: r.currency,
        account_id: r.account_id.map(|u| u.to_string()),
        tax_rate_id: r.tax_rate_id.map(|u| u.to_string()),
        category_id: r.category_id.map(|u| u.to_string()),
        is_active: r.is_active,
        is_bundle: r.is_bundle,
        bundle_components,
        created_at: r.created_at,
        updated_at: r.updated_at,
    })
}

const COLS: &str = "id, organization_id, name, description, sku, unit_price, currency, \
     account_id, tax_rate_id, category_id, is_active, is_bundle, created_at, updated_at";

pub struct ProductRepo;

impl ProductRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        category_id: Option<&str>,
    ) -> Result<Vec<Product>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<ProductRow> = if let Some(cat) = category_id {
            let cat_uuid = parse_uuid(cat)?;
            sqlx::query_as(&format!(
                "SELECT {COLS} FROM products \
                 WHERE organization_id = $1 AND category_id = $2 ORDER BY name"
            ))
            .bind(org_uuid)
            .bind(cat_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(&format!(
                "SELECT {COLS} FROM products WHERE organization_id = $1 ORDER BY name"
            ))
            .bind(org_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        };
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(product_from_row(pool, r).await?);
        }
        Ok(out)
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
        product_from_row(pool, row).await
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateProduct,
    ) -> Result<Product, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let acct_uuid = input.account_id.as_deref().map(parse_uuid).transpose()?;
        let tax_uuid = input.tax_rate_id.as_deref().map(parse_uuid).transpose()?;
        let cat_uuid = input.category_id.as_deref().map(parse_uuid).transpose()?;

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO products \
             (organization_id, name, description, sku, unit_price, currency, \
              account_id, tax_rate_id, category_id) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING id",
        )
        .bind(org_uuid)
        .bind(&input.name)
        .bind(&input.description)
        .bind(&input.sku)
        .bind(input.unit_price)
        .bind(&input.currency)
        .bind(acct_uuid)
        .bind(tax_uuid)
        .bind(cat_uuid)
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
        let cat_uuid = input.category_id.as_deref().map(parse_uuid).transpose()?;

        let n = sqlx::query(
            "UPDATE products SET \
             name        = COALESCE($1, name), \
             description = COALESCE($2, description), \
             sku         = COALESCE($3, sku), \
             unit_price  = COALESCE($4, unit_price), \
             account_id  = COALESCE($5, account_id), \
             tax_rate_id = COALESCE($6, tax_rate_id), \
             category_id = COALESCE($7, category_id), \
             is_active   = COALESCE($8, is_active), \
             is_bundle   = COALESCE($9, is_bundle), \
             updated_at  = NOW() \
             WHERE id = $10 AND organization_id = $11",
        )
        .bind(input.name)
        .bind(input.description)
        .bind(input.sku)
        .bind(input.unit_price)
        .bind(acct_uuid)
        .bind(tax_uuid)
        .bind(cat_uuid)
        .bind(input.is_active)
        .bind(input.is_bundle)
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

    /// Replace all bundle components for a product (idempotent full replace).
    pub async fn set_bundle_components(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: SetBundleComponents,
    ) -> Result<Product, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        // Verify product exists and belongs to org
        let is_bundle: Option<bool> = sqlx::query_scalar(
            "SELECT is_bundle FROM products WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        match is_bundle {
            None => return Err(DbError::NotFound),
            Some(false) => {
                return Err(DbError::Conflict(
                    "product must be marked as a bundle (is_bundle = true) first".into(),
                ))
            }
            Some(true) => {}
        }

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        sqlx::query("DELETE FROM product_bundle_components WHERE product_id = $1")
            .bind(id_uuid)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;

        for (i, comp) in input.components.iter().enumerate() {
            let comp_id = parse_uuid(&comp.component_id)?;
            // Prevent circular bundles
            if comp_id == id_uuid {
                return Err(DbError::Conflict(
                    "a bundle cannot include itself as a component".into(),
                ));
            }
            // Verify component belongs to same org
            let exists: Option<(Uuid,)> =
                sqlx::query_as("SELECT id FROM products WHERE organization_id = $1 AND id = $2")
                    .bind(org_uuid)
                    .bind(comp_id)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(map_sqlx_err)?;
            if exists.is_none() {
                return Err(DbError::NotFound);
            }
            sqlx::query(
                "INSERT INTO product_bundle_components
                    (product_id, component_id, quantity, sort_order)
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(id_uuid)
            .bind(comp_id)
            .bind(comp.quantity)
            .bind(comp.sort_order.max(i as i32))
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;
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
