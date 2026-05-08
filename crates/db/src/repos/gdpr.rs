use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

pub struct GdprRepo;

impl GdprRepo {
    /// Export all data held for a contact as a JSON bundle.
    pub async fn export_contact_data(
        pool: &PgPool,
        org_id: &str,
        contact_id: &str,
    ) -> Result<serde_json::Value, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = parse_uuid(contact_id)?;

        // Contact record
        let contact: Option<serde_json::Value> = sqlx::query_scalar(
            "SELECT row_to_json(c) FROM contacts c \
             WHERE id = $1 AND organization_id = $2",
        )
        .bind(contact_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        if contact.is_none() {
            return Err(DbError::NotFound);
        }

        // Invoices (and bills) for the contact
        let invoices: Vec<serde_json::Value> = sqlx::query_scalar(
            "SELECT row_to_json(i) FROM invoices i \
             WHERE contact_id = $1 AND organization_id = $2 ORDER BY created_at DESC",
        )
        .bind(contact_uuid)
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        // Payments against those invoices
        let payments: Vec<serde_json::Value> = sqlx::query_scalar(
            "SELECT row_to_json(p) FROM payments p \
             WHERE p.invoice_id IN \
               (SELECT id FROM invoices WHERE contact_id = $1 AND organization_id = $2) \
             ORDER BY payment_date DESC",
        )
        .bind(contact_uuid)
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        // Audit events referencing this contact
        let audit_events: Vec<serde_json::Value> = sqlx::query_scalar(
            "SELECT row_to_json(a) FROM audit_events a \
             WHERE organization_id = $1 AND resource_type = 'contact' AND resource_id = $2::text \
             ORDER BY created_at DESC LIMIT 1000",
        )
        .bind(org_uuid)
        .bind(contact_id)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(serde_json::json!({
            "contact": contact,
            "invoices": invoices,
            "payments": payments,
            "audit_events": audit_events,
        }))
    }

    /// Anonymise all PII for a contact and log the request.
    pub async fn forget_contact(
        pool: &PgPool,
        org_id: &str,
        contact_id: &str,
        requested_by: &str,
    ) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = parse_uuid(contact_id)?;
        let requester_uuid = parse_uuid(requested_by)?;

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        let n = sqlx::query(
            "UPDATE contacts SET \
               name       = 'ANONYMIZED-' || substring(id::text, 1, 8), \
               email      = NULL, \
               phone      = NULL, \
               address    = NULL, \
               tax_number = NULL, \
               tax_id     = NULL, \
               updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2",
        )
        .bind(contact_uuid)
        .bind(org_uuid)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if n == 0 {
            tx.rollback().await.map_err(map_sqlx_err)?;
            return Err(DbError::NotFound);
        }

        sqlx::query(
            "INSERT INTO gdpr_forget_requests \
             (organization_id, contact_id, requested_by) \
             VALUES ($1, $2, $3)",
        )
        .bind(org_uuid)
        .bind(contact_uuid)
        .bind(requester_uuid)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(())
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
