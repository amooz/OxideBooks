use oxidebooks_core::models::{
    Contractor1099Summary, ContractorTaxInfo, CreateContractorTaxInfo, UpdateContractorTaxInfo,
};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct TaxInfoRow {
    id: Uuid,
    organization_id: Uuid,
    contact_id: Uuid,
    tax_id_type: String,
    tax_id_last4: String,
    business_type: String,
    form_1099_type: String,
    w9_received_date: Option<Date>,
    notes: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn from_row(r: TaxInfoRow) -> ContractorTaxInfo {
    ContractorTaxInfo {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        contact_id: r.contact_id.to_string(),
        tax_id_type: r.tax_id_type,
        tax_id_last4: r.tax_id_last4,
        business_type: r.business_type,
        form_1099_type: r.form_1099_type,
        w9_received_date: r.w9_received_date,
        notes: r.notes,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

const COLS: &str = "id, organization_id, contact_id, tax_id_type, tax_id_last4, \
                    business_type, form_1099_type, w9_received_date, notes, \
                    created_at, updated_at";

pub struct ContractorTaxInfoRepo;

impl ContractorTaxInfoRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<ContractorTaxInfo>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<TaxInfoRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM contractor_tax_info \
             WHERE organization_id = $1 ORDER BY created_at ASC"
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
    ) -> Result<ContractorTaxInfo, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: TaxInfoRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM contractor_tax_info \
             WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(from_row(row))
    }

    pub async fn get_by_contact(
        pool: &PgPool,
        org_id: &str,
        contact_id: &str,
    ) -> Result<ContractorTaxInfo, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = parse_uuid(contact_id)?;
        let row: TaxInfoRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM contractor_tax_info \
             WHERE organization_id = $1 AND contact_id = $2"
        ))
        .bind(org_uuid)
        .bind(contact_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(from_row(row))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateContractorTaxInfo,
    ) -> Result<ContractorTaxInfo, DbError> {
        if input.tax_id_last4.len() != 4 || !input.tax_id_last4.chars().all(|c| c.is_ascii_digit())
        {
            return Err(DbError::Conflict(
                "tax_id_last4 must be exactly 4 digits".into(),
            ));
        }

        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = parse_uuid(&input.contact_id)?;
        let tax_id_type = input.tax_id_type.unwrap_or_else(|| "ein".into());
        let business_type = input.business_type.unwrap_or_else(|| "individual".into());
        let form_1099_type = input.form_1099_type.unwrap_or_else(|| "NEC".into());

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO contractor_tax_info \
             (organization_id, contact_id, tax_id_type, tax_id_last4, business_type, \
              form_1099_type, w9_received_date, notes) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING id",
        )
        .bind(org_uuid)
        .bind(contact_uuid)
        .bind(&tax_id_type)
        .bind(&input.tax_id_last4)
        .bind(&business_type)
        .bind(&form_1099_type)
        .bind(input.w9_received_date)
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
        input: UpdateContractorTaxInfo,
    ) -> Result<ContractorTaxInfo, DbError> {
        if let Some(ref last4) = input.tax_id_last4 {
            if last4.len() != 4 || !last4.chars().all(|c| c.is_ascii_digit()) {
                return Err(DbError::Conflict(
                    "tax_id_last4 must be exactly 4 digits".into(),
                ));
            }
        }

        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        sqlx::query(
            "UPDATE contractor_tax_info SET \
             tax_id_type      = COALESCE($1, tax_id_type), \
             tax_id_last4     = COALESCE($2, tax_id_last4), \
             business_type    = COALESCE($3, business_type), \
             form_1099_type   = COALESCE($4, form_1099_type), \
             w9_received_date = COALESCE($5, w9_received_date), \
             notes            = COALESCE($6, notes), \
             updated_at       = NOW() \
             WHERE organization_id = $7 AND id = $8",
        )
        .bind(input.tax_id_type)
        .bind(input.tax_id_last4)
        .bind(input.business_type)
        .bind(input.form_1099_type)
        .bind(input.w9_received_date)
        .bind(input.notes)
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, id).await
    }

    /// Returns 1099 payment totals per contractor for the given calendar year.
    /// Aggregates bill payments to contacts that have contractor_tax_info records.
    /// `threshold` is in minor units (e.g. 60000 = $600.00).
    pub async fn list_1099_payments(
        pool: &PgPool,
        org_id: &str,
        year: i32,
        threshold: i64,
    ) -> Result<Vec<Contractor1099Summary>, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        #[derive(sqlx::FromRow)]
        struct SummaryRow {
            contact_id: Uuid,
            contact_name: String,
            form_1099_type: String,
            total_paid: i64,
        }

        let rows: Vec<SummaryRow> = sqlx::query_as(
            "SELECT \
               c.id                  AS contact_id, \
               c.name                AS contact_name, \
               cti.form_1099_type    AS form_1099_type, \
               COALESCE(SUM(bp.amount), 0) AS total_paid \
             FROM contractor_tax_info cti \
             JOIN contacts c ON c.id = cti.contact_id \
             LEFT JOIN vendor_bills vb \
               ON vb.contact_id = cti.contact_id \
              AND vb.organization_id = $1 \
             LEFT JOIN bill_payments bp \
               ON bp.bill_id = vb.id \
              AND EXTRACT(YEAR FROM bp.payment_date) = $2 \
             WHERE cti.organization_id = $1 \
             GROUP BY c.id, c.name, cti.form_1099_type \
             ORDER BY c.name ASC",
        )
        .bind(org_uuid)
        .bind(year)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows
            .into_iter()
            .map(|r| Contractor1099Summary {
                contact_id: r.contact_id.to_string(),
                contact_name: r.contact_name,
                form_1099_type: r.form_1099_type,
                total_paid: r.total_paid,
                meets_threshold: r.total_paid >= threshold,
            })
            .collect())
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
