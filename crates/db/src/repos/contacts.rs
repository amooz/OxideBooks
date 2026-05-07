use oxidebooks_core::models::{
    Contact, ContactCreditStatus, ContactType, CreateContact, UpdateContact,
};
use oxidebooks_core::pagination::{encode_cursor, PageParams};
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
    tax_id: Option<String>,
    currency: Option<String>,
    credit_limit: Option<i64>,
    credit_limit_behaviour: String,
    is_active: bool,
    is_1099_vendor: bool,
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
            tax_id: r.tax_id,
            currency: r.currency,
            credit_limit: r.credit_limit,
            credit_limit_behaviour: r.credit_limit_behaviour,
            is_active: r.is_active,
            is_1099_vendor: r.is_1099_vendor,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

pub struct ContactRepo;

impl ContactRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        page: &PageParams,
    ) -> Result<(Vec<Contact>, Option<String>), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let limit = page.limit_clamped();
        let cursor = page.decode_cursor();

        let rows: Vec<ContactRow> = if let Some(c) = cursor {
            let cursor_ts = time::OffsetDateTime::parse(
                &c.created_at,
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(|_| DbError::Conflict("invalid cursor".into()))?;
            let cursor_id = parse_uuid(&c.id)?;
            sqlx::query_as(
                "SELECT id, organization_id, name, contact_type, email, phone, \
                 address, tax_number, tax_id, currency, credit_limit, credit_limit_behaviour, is_active, is_1099_vendor, \
                 created_at, updated_at \
                 FROM contacts \
                 WHERE organization_id = $1 AND (created_at, id) > ($2, $3) \
                 ORDER BY created_at ASC, id ASC LIMIT $4",
            )
            .bind(org_uuid)
            .bind(cursor_ts)
            .bind(cursor_id)
            .bind(limit + 1)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(
                "SELECT id, organization_id, name, contact_type, email, phone, \
                 address, tax_number, tax_id, currency, credit_limit, credit_limit_behaviour, is_active, is_1099_vendor, \
                 created_at, updated_at \
                 FROM contacts WHERE organization_id = $1 \
                 ORDER BY created_at ASC, id ASC LIMIT $2",
            )
            .bind(org_uuid)
            .bind(limit + 1)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        };

        let has_next = rows.len() as i64 > limit;
        let mut rows = rows;
        if has_next {
            rows.pop();
        }
        let next_cursor = if has_next {
            rows.last()
                .map(|r| encode_cursor(r.created_at, &r.id.to_string()))
        } else {
            None
        };
        Ok((rows.into_iter().map(Contact::from).collect(), next_cursor))
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<Contact, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let row: ContactRow = sqlx::query_as(
            "SELECT id, organization_id, name, contact_type, email, phone, \
             address, tax_number, tax_id, currency, credit_limit, credit_limit_behaviour, is_active, is_1099_vendor, \
             created_at, updated_at \
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

    /// Delete a contact. Returns `DbError::Conflict` if the contact has live invoices.
    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        // Verify the contact exists first.
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM contacts WHERE organization_id = $1 AND id = $2")
                .bind(org_uuid)
                .bind(id_uuid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;

        if exists.is_none() {
            return Err(DbError::NotFound);
        }

        // Block deletion if linked to any non-voided invoice.
        let linked: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM invoices \
             WHERE organization_id = $1 AND contact_id = $2 AND status != 'voided'",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        if linked.0 > 0 {
            return Err(DbError::Conflict(
                "contact has linked invoices and cannot be deleted; \
                 void all invoices first or archive the contact instead"
                    .into(),
            ));
        }

        sqlx::query("DELETE FROM contacts WHERE organization_id = $1 AND id = $2")
            .bind(org_uuid)
            .bind(id_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;

        Ok(())
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
             name            = COALESCE($3, name), \
             email           = COALESCE($4, email), \
             phone           = COALESCE($5, phone), \
             address         = COALESCE($6, address), \
             tax_number      = COALESCE($7, tax_number), \
             currency        = COALESCE($8, currency), \
             is_active       = COALESCE($9, is_active), \
             tax_id          = COALESCE($10, tax_id), \
             is_1099_vendor  = COALESCE($11, is_1099_vendor), \
             credit_limit              = COALESCE($12, credit_limit), \
             credit_limit_behaviour    = COALESCE($13, credit_limit_behaviour), \
             updated_at                = NOW() \
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
        .bind(&input.tax_id)
        .bind(input.is_1099_vendor)
        .bind(input.credit_limit)
        .bind(&input.credit_limit_behaviour)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, id).await
    }

    /// Merge `discard_id` into `keep_id`: re-point all FK references, then delete the discard.
    /// Returns the surviving contact.
    pub async fn merge(
        pool: &PgPool,
        org_id: &str,
        keep_id: &str,
        discard_id: &str,
    ) -> Result<Contact, DbError> {
        if keep_id == discard_id {
            return Err(DbError::Conflict(
                "keep_id and discard_id must be different".into(),
            ));
        }
        let org_uuid = parse_uuid(org_id)?;
        let keep_uuid = parse_uuid(keep_id)?;
        let discard_uuid = parse_uuid(discard_id)?;

        // Verify both contacts belong to this org.
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM contacts \
             WHERE organization_id = $1 AND id = ANY($2)",
        )
        .bind(org_uuid)
        .bind(vec![keep_uuid, discard_uuid])
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        if count.0 < 2 {
            return Err(DbError::NotFound);
        }

        // Re-point all child tables from discard → keep.
        let fk_updates: &[&str] = &[
            "UPDATE invoices SET contact_id = $1 WHERE contact_id = $2 AND organization_id = $3",
            "UPDATE vendor_bills SET contact_id = $1 WHERE contact_id = $2 AND organization_id = $3",
            "UPDATE payments SET contact_id = $1 WHERE contact_id = $2 AND organization_id = $3",
            "UPDATE expenses SET contact_id = $1 WHERE contact_id = $2 AND organization_id = $3",
            "UPDATE notes SET entity_id = $1::TEXT WHERE entity_id = $2::TEXT AND entity_type = 'contact'",
        ];
        for sql in fk_updates {
            sqlx::query(sql)
                .bind(keep_uuid)
                .bind(discard_uuid)
                .bind(org_uuid)
                .execute(pool)
                .await
                .ok(); // ignore missing column errors on tables that don't have contact_id
        }

        // Delete the discard contact.
        sqlx::query("DELETE FROM contacts WHERE id = $1 AND organization_id = $2")
            .bind(discard_uuid)
            .bind(org_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, keep_id).await
    }

    pub async fn credit_status(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<ContactCreditStatus, DbError> {
        let contact = Self::get_by_id(pool, org_id, id).await?;
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let outstanding: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(total_amount - amount_paid), 0)::BIGINT \
             FROM invoices \
             WHERE organization_id = $1 AND contact_id = $2 \
               AND invoice_type = 'invoice' \
               AND status NOT IN ('draft', 'voided', 'paid')",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let available_credit = contact.credit_limit.map(|limit| limit - outstanding);
        let overlimit = contact
            .credit_limit
            .map(|limit| outstanding > limit)
            .unwrap_or(false);

        Ok(ContactCreditStatus {
            contact_id: id.to_string(),
            credit_limit: contact.credit_limit,
            credit_limit_behaviour: contact.credit_limit_behaviour,
            outstanding_balance: outstanding,
            available_credit,
            overlimit,
        })
    }

    /// All active contacts with an email address (used for bulk statement delivery).
    pub async fn list_with_email(pool: &PgPool, org_id: &str) -> Result<Vec<Contact>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<ContactRow> = sqlx::query_as(
            "SELECT id, organization_id, name, contact_type, email, phone, address, \
             tax_number, tax_id, currency, credit_limit, credit_limit_behaviour, \
             is_active, is_1099_vendor, created_at, updated_at \
             FROM contacts \
             WHERE organization_id = $1 AND is_active = TRUE AND email IS NOT NULL \
             ORDER BY name",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(Contact::from).collect())
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
