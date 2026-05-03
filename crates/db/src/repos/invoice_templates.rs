use oxidebooks_core::models::{InvoiceTemplate, UpsertInvoiceTemplate};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct TemplateRow {
    id: Uuid,
    organization_id: Uuid,
    logo_url: Option<String>,
    accent_color: Option<String>,
    footer_text: Option<String>,
    default_payment_terms_days: i32,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn from_row(r: TemplateRow) -> InvoiceTemplate {
    InvoiceTemplate {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        logo_url: r.logo_url,
        accent_color: r.accent_color,
        footer_text: r.footer_text,
        default_payment_terms_days: r.default_payment_terms_days,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

pub struct InvoiceTemplateRepo;

impl InvoiceTemplateRepo {
    pub async fn get(pool: &PgPool, org_id: &str) -> Result<InvoiceTemplate, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let row: TemplateRow = sqlx::query_as(
            "SELECT id, organization_id, logo_url, accent_color, footer_text, \
             default_payment_terms_days, created_at, updated_at \
             FROM invoice_templates WHERE organization_id = $1",
        )
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(from_row(row))
    }

    pub async fn upsert(
        pool: &PgPool,
        org_id: &str,
        input: UpsertInvoiceTemplate,
    ) -> Result<InvoiceTemplate, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let row: TemplateRow = sqlx::query_as(
            "INSERT INTO invoice_templates \
             (organization_id, logo_url, accent_color, footer_text, default_payment_terms_days) \
             VALUES ($1,$2,$3,$4,$5) \
             ON CONFLICT (organization_id) DO UPDATE SET \
               logo_url                    = EXCLUDED.logo_url, \
               accent_color                = EXCLUDED.accent_color, \
               footer_text                 = EXCLUDED.footer_text, \
               default_payment_terms_days  = EXCLUDED.default_payment_terms_days, \
               updated_at                  = NOW() \
             RETURNING id, organization_id, logo_url, accent_color, footer_text, \
                       default_payment_terms_days, created_at, updated_at",
        )
        .bind(org_uuid)
        .bind(&input.logo_url)
        .bind(&input.accent_color)
        .bind(&input.footer_text)
        .bind(input.default_payment_terms_days)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(from_row(row))
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
