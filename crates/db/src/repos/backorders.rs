use oxidebooks_core::models::{
    Backorder, CreateBackorder, CreateDropShipRequest, DropShipRequest, FulfillBackorder,
    UpdateDropShipRequest,
};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

fn parse_date(s: &str) -> Result<Date, DbError> {
    Date::parse(s, &time::format_description::well_known::Iso8601::DEFAULT)
        .map_err(|_| DbError::Conflict(format!("invalid date: {s}")))
}

#[derive(sqlx::FromRow)]
struct BackorderRow {
    id: Uuid,
    organization_id: Uuid,
    so_id: Uuid,
    so_line_id: Uuid,
    product_id: Option<Uuid>,
    quantity: i64,
    status: String,
    expected_date: Option<Date>,
    fulfilled_at: Option<OffsetDateTime>,
    notes: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn backorder_from_row(r: BackorderRow) -> Backorder {
    Backorder {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        so_id: r.so_id.to_string(),
        so_line_id: r.so_line_id.to_string(),
        product_id: r.product_id.map(|u| u.to_string()),
        quantity: r.quantity,
        status: r.status,
        expected_date: r.expected_date,
        fulfilled_at: r.fulfilled_at,
        notes: r.notes,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

const BO_COLS: &str = "id, organization_id, so_id, so_line_id, product_id, quantity, status, \
     expected_date, fulfilled_at, notes, created_at, updated_at";

#[derive(sqlx::FromRow)]
struct DropShipRow {
    id: Uuid,
    organization_id: Uuid,
    so_id: Uuid,
    so_line_id: Uuid,
    po_id: Option<Uuid>,
    vendor_id: Uuid,
    product_id: Option<Uuid>,
    quantity: i64,
    status: String,
    ship_to_name: Option<String>,
    ship_to_address: Option<String>,
    notes: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn dropship_from_row(r: DropShipRow) -> DropShipRequest {
    DropShipRequest {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        so_id: r.so_id.to_string(),
        so_line_id: r.so_line_id.to_string(),
        po_id: r.po_id.map(|u| u.to_string()),
        vendor_id: r.vendor_id.to_string(),
        product_id: r.product_id.map(|u| u.to_string()),
        quantity: r.quantity,
        status: r.status,
        ship_to_name: r.ship_to_name,
        ship_to_address: r.ship_to_address,
        notes: r.notes,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

const DS_COLS: &str =
    "id, organization_id, so_id, so_line_id, po_id, vendor_id, product_id, quantity, status, \
     ship_to_name, ship_to_address, notes, created_at, updated_at";

pub struct BackorderRepo;

impl BackorderRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        status: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Backorder>, DbError> {
        let org = parse_uuid(org_id)?;
        let rows: Vec<BackorderRow> = if let Some(s) = status {
            sqlx::query_as(&format!(
                "SELECT {BO_COLS} FROM backorders \
                 WHERE organization_id = $1 AND status = $2 \
                 ORDER BY created_at DESC LIMIT $3"
            ))
            .bind(org)
            .bind(s)
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(&format!(
                "SELECT {BO_COLS} FROM backorders \
                 WHERE organization_id = $1 \
                 ORDER BY created_at DESC LIMIT $2"
            ))
            .bind(org)
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        };
        Ok(rows.into_iter().map(backorder_from_row).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<Backorder, DbError> {
        let org = parse_uuid(org_id)?;
        let bid = parse_uuid(id)?;
        let row: Option<BackorderRow> = sqlx::query_as(&format!(
            "SELECT {BO_COLS} FROM backorders WHERE id = $1 AND organization_id = $2"
        ))
        .bind(bid)
        .bind(org)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;
        row.map(backorder_from_row).ok_or(DbError::NotFound)
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateBackorder,
    ) -> Result<Backorder, DbError> {
        let org = parse_uuid(org_id)?;
        let so_id = parse_uuid(&input.so_id)?;
        let so_line_id = parse_uuid(&input.so_line_id)?;
        let product_id = input.product_id.as_deref().map(parse_uuid).transpose()?;
        let expected_date = input.expected_date.as_deref().map(parse_date).transpose()?;

        if input.quantity <= 0 {
            return Err(DbError::Conflict("quantity must be positive".into()));
        }

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO backorders \
             (organization_id, so_id, so_line_id, product_id, quantity, expected_date, notes) \
             VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id",
        )
        .bind(org)
        .bind(so_id)
        .bind(so_line_id)
        .bind(product_id)
        .bind(input.quantity)
        .bind(expected_date)
        .bind(&input.notes)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        // Increment quantity_backordered on the SO line.
        sqlx::query(
            "UPDATE sales_order_lines \
             SET quantity_backordered = quantity_backordered + $1 \
             WHERE id = $2",
        )
        .bind(input.quantity)
        .bind(so_line_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn fulfill(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: FulfillBackorder,
    ) -> Result<Backorder, DbError> {
        let org = parse_uuid(org_id)?;
        let bid = parse_uuid(id)?;

        let row = Self::get_by_id(pool, org_id, id).await?;
        if row.status != "pending" {
            return Err(DbError::Conflict(
                "only pending backorders can be fulfilled".into(),
            ));
        }

        let so_line_id = parse_uuid(&row.so_line_id)?;

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        let n = sqlx::query(
            "UPDATE backorders \
             SET status = 'fulfilled', fulfilled_at = NOW(), \
                 notes = COALESCE($2, notes), updated_at = NOW() \
             WHERE id = $1 AND organization_id = $3 AND status = 'pending'",
        )
        .bind(bid)
        .bind(&input.notes)
        .bind(org)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if n == 0 {
            return Err(DbError::NotFound);
        }

        // Decrement quantity_backordered on the SO line.
        sqlx::query(
            "UPDATE sales_order_lines \
             SET quantity_backordered = GREATEST(quantity_backordered - $1, 0) \
             WHERE id = $2",
        )
        .bind(row.quantity)
        .bind(so_line_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn cancel(pool: &PgPool, org_id: &str, id: &str) -> Result<Backorder, DbError> {
        let org = parse_uuid(org_id)?;
        let bid = parse_uuid(id)?;

        let row = Self::get_by_id(pool, org_id, id).await?;
        if row.status != "pending" {
            return Err(DbError::Conflict(
                "only pending backorders can be cancelled".into(),
            ));
        }

        let so_line_id = parse_uuid(&row.so_line_id)?;

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        sqlx::query(
            "UPDATE backorders \
             SET status = 'cancelled', updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2",
        )
        .bind(bid)
        .bind(org)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        sqlx::query(
            "UPDATE sales_order_lines \
             SET quantity_backordered = GREATEST(quantity_backordered - $1, 0) \
             WHERE id = $2",
        )
        .bind(row.quantity)
        .bind(so_line_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, id).await
    }

    // ── Drop-ship requests ─────────────────────────────────────────────────────

    pub async fn list_drop_ships(
        pool: &PgPool,
        org_id: &str,
        status: Option<&str>,
        limit: i64,
    ) -> Result<Vec<DropShipRequest>, DbError> {
        let org = parse_uuid(org_id)?;
        let rows: Vec<DropShipRow> = if let Some(s) = status {
            sqlx::query_as(&format!(
                "SELECT {DS_COLS} FROM drop_ship_requests \
                 WHERE organization_id = $1 AND status = $2 \
                 ORDER BY created_at DESC LIMIT $3"
            ))
            .bind(org)
            .bind(s)
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(&format!(
                "SELECT {DS_COLS} FROM drop_ship_requests \
                 WHERE organization_id = $1 \
                 ORDER BY created_at DESC LIMIT $2"
            ))
            .bind(org)
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        };
        Ok(rows.into_iter().map(dropship_from_row).collect())
    }

    pub async fn get_drop_ship(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<DropShipRequest, DbError> {
        let org = parse_uuid(org_id)?;
        let did = parse_uuid(id)?;
        let row: Option<DropShipRow> = sqlx::query_as(&format!(
            "SELECT {DS_COLS} FROM drop_ship_requests WHERE id = $1 AND organization_id = $2"
        ))
        .bind(did)
        .bind(org)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;
        row.map(dropship_from_row).ok_or(DbError::NotFound)
    }

    pub async fn create_drop_ship(
        pool: &PgPool,
        org_id: &str,
        input: CreateDropShipRequest,
    ) -> Result<DropShipRequest, DbError> {
        let org = parse_uuid(org_id)?;
        let so_id = parse_uuid(&input.so_id)?;
        let so_line_id = parse_uuid(&input.so_line_id)?;
        let vendor_id = parse_uuid(&input.vendor_id)?;
        let product_id = input.product_id.as_deref().map(parse_uuid).transpose()?;

        if input.quantity <= 0 {
            return Err(DbError::Conflict("quantity must be positive".into()));
        }

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO drop_ship_requests \
             (organization_id, so_id, so_line_id, vendor_id, product_id, quantity, \
              ship_to_name, ship_to_address, notes) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING id",
        )
        .bind(org)
        .bind(so_id)
        .bind(so_line_id)
        .bind(vendor_id)
        .bind(product_id)
        .bind(input.quantity)
        .bind(&input.ship_to_name)
        .bind(&input.ship_to_address)
        .bind(&input.notes)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_drop_ship(pool, org_id, &id.to_string()).await
    }

    pub async fn update_drop_ship(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateDropShipRequest,
    ) -> Result<DropShipRequest, DbError> {
        let org = parse_uuid(org_id)?;
        let did = parse_uuid(id)?;
        let po_id = input.po_id.as_deref().map(parse_uuid).transpose()?;

        if let Some(s) = &input.status {
            let valid = [
                "requested",
                "po_created",
                "shipped",
                "delivered",
                "cancelled",
            ];
            if !valid.contains(&s.as_str()) {
                return Err(DbError::Conflict(format!("invalid drop-ship status: {s}")));
            }
        }

        let n = sqlx::query(
            "UPDATE drop_ship_requests SET \
             po_id           = COALESCE($3, po_id), \
             status          = COALESCE($4, status), \
             ship_to_name    = COALESCE($5, ship_to_name), \
             ship_to_address = COALESCE($6, ship_to_address), \
             notes           = COALESCE($7, notes), \
             updated_at      = NOW() \
             WHERE id = $1 AND organization_id = $2",
        )
        .bind(did)
        .bind(org)
        .bind(po_id)
        .bind(&input.status)
        .bind(&input.ship_to_name)
        .bind(&input.ship_to_address)
        .bind(&input.notes)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if n == 0 {
            return Err(DbError::NotFound);
        }

        Self::get_drop_ship(pool, org_id, id).await
    }
}
