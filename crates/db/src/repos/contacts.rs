use oxidebooks_core::models::{Contact, ContactType, CreateContact, UpdateContact};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct ContactRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    contact_type: String,
    email: Option<String>,
    phone: Option<String>,
    address: Option<String>,
    tax_number: Option<String>,
    currency: Option<String>,
    is_active: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<ContactRow> for Contact {
    fn from(r: ContactRow) -> Self {
        Contact {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            name: r.name,
            contact_type: match r.contact_type.as_str() {
                "customer" => ContactType::Customer,
                "vendor" => ContactType::Vendor,
                _ => ContactType::Both,
            },
            email: r.email,
            phone: r.phone,
            address: r.address,
            tax_number: r.tax_number,
            currency: r.currency,
            is_active: r.is_active,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

pub struct ContactRepo;

impl ContactRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<Contact>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<ContactRow> = sqlx::query_as(
            "SELECT id, organization_id, name, contact_type, email, phone, \
             address, tax_number, currency, is_active, created_at, updated_at \
             FROM contacts WHERE organization_id = $1 ORDER BY name",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows.into_iter().map(Contact::from).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<Contact, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let row: ContactRow = sqlx::query_as(
            "SELECT id, organization_id, name, contact_type, email, phone, \
             address, tax_number, currency, is_active, created_at, updated_at \
             FROM contacts WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        Ok(row.into())
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateContact,
    ) -> Result<Contact, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id = Uuid::new_v4();
        let contact_type = input.contact_type.unwrap_or(ContactType::Both).to_string();

        sqlx::query(
            "INSERT INTO contacts \
             (id, organization_id, name, contact_type, email, phone, address, tax_number, currency) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(&input.name)
        .bind(&contact_type)
        .bind(&input.email)
        .bind(&input.phone)
        .bind(&input.address)
        .bind(&input.tax_number)
        .bind(&input.currency)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateContact,
    ) -> Result<Contact, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        sqlx::query(
            "UPDATE contacts SET \
             name       = COALESCE($3, name), \
             email      = COALESCE($4, email), \
             phone      = COALESCE($5, phone), \
             address    = COALESCE($6, address), \
             tax_number = COALESCE($7, tax_number), \
             currency   = COALESCE($8, currency), \
             is_active  = COALESCE($9, is_active), \
             updated_at = NOW() \
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .bind(&input.name)
        .bind(&input.email)
        .bind(&input.phone)
        .bind(&input.address)
        .bind(&input.tax_number)
        .bind(&input.currency)
        .bind(input.is_active)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, id).await
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
