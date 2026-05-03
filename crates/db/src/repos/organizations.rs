use oxidebooks_core::models::{CreateOrganization, Organization, UpdateOrganization};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct OrgRow {
    id: Uuid,
    name: String,
    currency: String,
    fiscal_year_start: i16,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

pub struct OrganizationRepo;

impl OrganizationRepo {
    pub async fn create(pool: &PgPool, input: CreateOrganization) -> Result<Organization, DbError> {
        let id = Uuid::new_v4();
        let fiscal_year_start = input.fiscal_year_start.unwrap_or(1) as i16;

        sqlx::query(
            "INSERT INTO organizations (id, name, currency, fiscal_year_start) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(id)
        .bind(&input.name)
        .bind(&input.currency)
        .bind(fiscal_year_start)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, id).await
    }

    pub async fn get_by_id_str(pool: &PgPool, id: &str) -> Result<Organization, DbError> {
        let uuid =
            Uuid::parse_str(id).map_err(|_| DbError::Conflict(format!("invalid UUID: {id}")))?;
        Self::get_by_id(pool, uuid).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        input: UpdateOrganization,
    ) -> Result<Organization, DbError> {
        let uuid = Uuid::parse_str(org_id)
            .map_err(|_| DbError::Conflict(format!("invalid UUID: {org_id}")))?;

        if let Some(ref name) = input.name {
            sqlx::query("UPDATE organizations SET name = $1, updated_at = NOW() WHERE id = $2")
                .bind(name)
                .bind(uuid)
                .execute(pool)
                .await
                .map_err(map_sqlx_err)?;
        }

        if let Some(ref currency) = input.currency {
            sqlx::query("UPDATE organizations SET currency = $1, updated_at = NOW() WHERE id = $2")
                .bind(currency)
                .bind(uuid)
                .execute(pool)
                .await
                .map_err(map_sqlx_err)?;
        }

        if let Some(fys) = input.fiscal_year_start {
            sqlx::query(
                "UPDATE organizations \
                 SET fiscal_year_start = $1, updated_at = NOW() \
                 WHERE id = $2",
            )
            .bind(fys as i16)
            .bind(uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;
        }

        Self::get_by_id(pool, uuid).await
    }

    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Organization, DbError> {
        let row: OrgRow = sqlx::query_as(
            "SELECT id, name, currency, fiscal_year_start, created_at, updated_at \
             FROM organizations WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        Ok(row.into())
    }
}

impl From<OrgRow> for Organization {
    fn from(r: OrgRow) -> Self {
        Organization {
            id: r.id.to_string(),
            name: r.name,
            currency: r.currency,
            fiscal_year_start: r.fiscal_year_start as u8,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
