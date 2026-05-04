use oxidebooks_core::models::{CreateSalesTaxNexus, SalesTaxNexus, UpdateSalesTaxNexus};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct NexusRow {
    id: Uuid,
    organization_id: Uuid,
    jurisdiction_code: String,
    jurisdiction_name: String,
    nexus_type: String,
    registration_number: Option<String>,
    effective_date: Date,
    end_date: Option<Date>,
    status: String,
    notes: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn from_row(r: NexusRow) -> SalesTaxNexus {
    SalesTaxNexus {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        jurisdiction_code: r.jurisdiction_code,
        jurisdiction_name: r.jurisdiction_name,
        nexus_type: r.nexus_type,
        registration_number: r.registration_number,
        effective_date: r.effective_date,
        end_date: r.end_date,
        status: r.status,
        notes: r.notes,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

const COLS: &str = "id, organization_id, jurisdiction_code, jurisdiction_name, nexus_type, \
                    registration_number, effective_date, end_date, status, notes, \
                    created_at, updated_at";

pub struct SalesTaxNexusRepo;

impl SalesTaxNexusRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<SalesTaxNexus>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<NexusRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM sales_tax_nexus \
             WHERE organization_id = $1 ORDER BY jurisdiction_code ASC"
        ))
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<SalesTaxNexus, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: NexusRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM sales_tax_nexus WHERE organization_id = $1 AND id = $2"
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
        input: CreateSalesTaxNexus,
    ) -> Result<SalesTaxNexus, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let nexus_type = input.nexus_type.unwrap_or_else(|| "physical".into());

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO sales_tax_nexus \
             (organization_id, jurisdiction_code, jurisdiction_name, nexus_type, \
              registration_number, effective_date, end_date, notes) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING id",
        )
        .bind(org_uuid)
        .bind(&input.jurisdiction_code)
        .bind(&input.jurisdiction_name)
        .bind(&nexus_type)
        .bind(&input.registration_number)
        .bind(input.effective_date)
        .bind(input.end_date)
        .bind(&input.notes)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateSalesTaxNexus,
    ) -> Result<SalesTaxNexus, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        sqlx::query(
            "UPDATE sales_tax_nexus SET \
             registration_number = COALESCE($1, registration_number), \
             end_date             = COALESCE($2, end_date), \
             status               = COALESCE($3, status), \
             notes                = COALESCE($4, notes), \
             updated_at           = NOW() \
             WHERE organization_id = $5 AND id = $6",
        )
        .bind(input.registration_number)
        .bind(input.end_date)
        .bind(input.status)
        .bind(input.notes)
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows =
            sqlx::query("DELETE FROM sales_tax_nexus WHERE organization_id = $1 AND id = $2")
                .bind(org_uuid)
                .bind(id_uuid)
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
