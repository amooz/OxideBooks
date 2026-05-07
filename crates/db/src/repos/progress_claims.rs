use oxidebooks_core::models::{
    CreateInvoice, CreateInvoiceLine, CreateProgressClaim, InvoiceType, ProgressClaim,
    ProjectBillingReport, ProjectBillingRow, ReleaseRetainage,
};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    error::{map_sqlx_err, DbError},
    repos::{InvoiceRepo, ProjectRepo},
};

#[derive(sqlx::FromRow)]
struct ClaimRow {
    id: Uuid,
    organization_id: Uuid,
    project_id: Uuid,
    claim_number: i32,
    claim_pct: i64,
    claim_amount: i64,
    retainage_pct: i64,
    retainage_amount: i64,
    net_amount: i64,
    status: String,
    notes: Option<String>,
    invoice_id: Option<Uuid>,
    approved_at: Option<OffsetDateTime>,
    invoiced_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn from_row(r: ClaimRow) -> ProgressClaim {
    ProgressClaim {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        project_id: r.project_id.to_string(),
        claim_number: r.claim_number,
        claim_pct: r.claim_pct,
        claim_amount: r.claim_amount,
        retainage_pct: r.retainage_pct,
        retainage_amount: r.retainage_amount,
        net_amount: r.net_amount,
        status: r.status,
        notes: r.notes,
        invoice_id: r.invoice_id.map(|u| u.to_string()),
        approved_at: r.approved_at,
        invoiced_at: r.invoiced_at,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

const COLS: &str = "id, organization_id, project_id, claim_number, claim_pct, claim_amount, \
     retainage_pct, retainage_amount, net_amount, status, notes, invoice_id, \
     approved_at, invoiced_at, created_at, updated_at";

pub struct ProgressClaimRepo;

impl ProgressClaimRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        project_id: &str,
    ) -> Result<Vec<ProgressClaim>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let project_uuid = parse_uuid(project_id)?;
        let rows: Vec<ClaimRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM project_progress_claims \
             WHERE organization_id = $1 AND project_id = $2 \
             ORDER BY claim_number"
        ))
        .bind(org_uuid)
        .bind(project_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        claim_id: &str,
    ) -> Result<ProgressClaim, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let claim_uuid = parse_uuid(claim_id)?;
        let row: ClaimRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM project_progress_claims \
             WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(claim_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(from_row(row))
    }

    /// Create a progress claim, validating cumulative % ≤ 100%.
    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        project_id: &str,
        input: CreateProgressClaim,
    ) -> Result<ProgressClaim, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let project_uuid = parse_uuid(project_id)?;

        // Validate cumulative claim percentage ≤ 100% (stored × 100, so max 10000)
        let existing_pct: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(claim_pct), 0)::BIGINT \
             FROM project_progress_claims \
             WHERE project_id = $1 AND organization_id = $2",
        )
        .bind(project_uuid)
        .bind(org_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        if existing_pct + input.claim_pct > 10_000 {
            return Err(DbError::Conflict(format!(
                "cumulative claim percentage would exceed 100% \
                 (already claimed {}%, requested {}%)",
                existing_pct / 100,
                input.claim_pct / 100
            )));
        }

        let claim_number: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(claim_number), 0) + 1 \
             FROM project_progress_claims \
             WHERE project_id = $1 AND organization_id = $2",
        )
        .bind(project_uuid)
        .bind(org_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let retainage_amount = input.claim_amount * input.retainage_pct / 10_000;
        let net_amount = input.claim_amount - retainage_amount;

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO project_progress_claims \
             (organization_id, project_id, claim_number, claim_pct, claim_amount, \
              retainage_pct, retainage_amount, net_amount, notes) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING id",
        )
        .bind(org_uuid)
        .bind(project_uuid)
        .bind(claim_number)
        .bind(input.claim_pct)
        .bind(input.claim_amount)
        .bind(input.retainage_pct)
        .bind(retainage_amount)
        .bind(net_amount)
        .bind(&input.notes)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    /// Approve a draft progress claim.
    pub async fn approve(
        pool: &PgPool,
        org_id: &str,
        claim_id: &str,
    ) -> Result<ProgressClaim, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let claim_uuid = parse_uuid(claim_id)?;

        let n = sqlx::query(
            "UPDATE project_progress_claims \
             SET status = 'approved', approved_at = NOW(), updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2 AND status = 'draft'",
        )
        .bind(claim_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if n == 0 {
            return Err(DbError::Conflict(
                "claim not found or not in draft status".into(),
            ));
        }
        Self::get_by_id(pool, org_id, claim_id).await
    }

    /// Convert an approved progress claim into an invoice (net of retainage).
    pub async fn convert_to_invoice(
        pool: &PgPool,
        org_id: &str,
        claim_id: &str,
    ) -> Result<ProgressClaim, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let claim_uuid = parse_uuid(claim_id)?;

        let claim = Self::get_by_id(pool, org_id, claim_id).await?;
        if claim.status != "approved" {
            return Err(DbError::Conflict(
                "claim must be approved before invoicing".into(),
            ));
        }

        let project = ProjectRepo::get_by_id(pool, org_id, &claim.project_id).await?;
        let contact_id = project
            .contact_id
            .ok_or_else(|| DbError::Conflict("project has no contact to invoice".into()))?;

        let today = OffsetDateTime::now_utc().date();
        let create_invoice = CreateInvoice {
            contact_id,
            invoice_type: InvoiceType::Invoice,
            date: today,
            due_date: today,
            currency: None,
            exchange_rate: None,
            notes: claim.notes.clone(),
            global_discount_pct: 0,
            lines: vec![CreateInvoiceLine {
                description: format!(
                    "Progress Claim #{} ({:.2}% of contract)",
                    claim.claim_number,
                    claim.claim_pct as f64 / 100.0
                ),
                account_id: None,
                quantity: 100,
                unit_price: claim.net_amount,
                tax_rate: None,
                discount_pct: 0,
                product_id: None,
                variant_id: None,
            }],
        };

        let invoice = InvoiceRepo::create(pool, org_id, create_invoice).await?;
        let invoice_uuid = parse_uuid(&invoice.id)?;

        sqlx::query(
            "UPDATE project_progress_claims \
             SET status = 'invoiced', invoice_id = $1, invoiced_at = NOW(), updated_at = NOW() \
             WHERE id = $2 AND organization_id = $3",
        )
        .bind(invoice_uuid)
        .bind(claim_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, claim_id).await
    }

    /// Release all withheld retainage for a project as a final invoice.
    pub async fn release_retainage(
        pool: &PgPool,
        org_id: &str,
        project_id: &str,
        input: ReleaseRetainage,
    ) -> Result<ProgressClaim, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let project_uuid = parse_uuid(project_id)?;

        // Guard against duplicate release: claim_pct = 0 marks a retainage-release row.
        let already_released: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM project_progress_claims \
             WHERE project_id = $1 AND organization_id = $2 AND claim_pct = 0 AND status = 'invoiced'",
        )
        .bind(project_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        if already_released.is_some() {
            return Err(DbError::Conflict(
                "retainage has already been released for this project".into(),
            ));
        }

        let total_retainage: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(retainage_amount), 0)::BIGINT \
             FROM project_progress_claims \
             WHERE project_id = $1 AND organization_id = $2 AND status = 'invoiced'",
        )
        .bind(project_uuid)
        .bind(org_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        if total_retainage == 0 {
            return Err(DbError::Conflict(
                "no retainage to release for this project".into(),
            ));
        }

        let project = ProjectRepo::get_by_id(pool, org_id, project_id).await?;
        let contact_id = project
            .contact_id
            .ok_or_else(|| DbError::Conflict("project has no contact to invoice".into()))?;

        let claim_number: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(claim_number), 0) + 1 \
             FROM project_progress_claims \
             WHERE project_id = $1 AND organization_id = $2",
        )
        .bind(project_uuid)
        .bind(org_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let today = OffsetDateTime::now_utc().date();
        let create_invoice = CreateInvoice {
            contact_id,
            invoice_type: InvoiceType::Invoice,
            date: today,
            due_date: today,
            currency: None,
            exchange_rate: None,
            notes: input.notes.clone(),
            global_discount_pct: 0,
            lines: vec![CreateInvoiceLine {
                description: "Retainage Release".to_string(),
                account_id: None,
                quantity: 100,
                unit_price: total_retainage,
                tax_rate: None,
                discount_pct: 0,
                product_id: None,
                variant_id: None,
            }],
        };

        let invoice = InvoiceRepo::create(pool, org_id, create_invoice).await?;
        let invoice_uuid = parse_uuid(&invoice.id)?;

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO project_progress_claims \
             (organization_id, project_id, claim_number, claim_pct, claim_amount, \
              retainage_pct, retainage_amount, net_amount, status, notes, \
              invoice_id, approved_at, invoiced_at) \
             VALUES ($1,$2,$3,0,$4,0,0,$4,'invoiced',$5,$6,NOW(),NOW()) RETURNING id",
        )
        .bind(org_uuid)
        .bind(project_uuid)
        .bind(claim_number)
        .bind(total_retainage)
        .bind(input.notes)
        .bind(invoice_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    /// Project billing summary: billable vs billed vs retainage per project.
    pub async fn project_billing_report(
        pool: &PgPool,
        org_id: &str,
    ) -> Result<ProjectBillingReport, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        #[derive(sqlx::FromRow)]
        struct BillingRow {
            project_id: Uuid,
            project_name: String,
            billing_method: String,
            contract_amount: i64,
            billed_amount: i64,
            retainage_held: i64,
        }

        let rows: Vec<BillingRow> = sqlx::query_as(
            "SELECT \
               p.id AS project_id, \
               p.name AS project_name, \
               p.billing_method, \
               COALESCE(p.budget_amount, 0) AS contract_amount, \
               COALESCE(SUM(CASE WHEN pc.status = 'invoiced' THEN pc.net_amount ELSE 0 END), 0)::BIGINT AS billed_amount, \
               COALESCE(SUM(CASE WHEN pc.status = 'invoiced' THEN pc.retainage_amount ELSE 0 END), 0)::BIGINT AS retainage_held \
             FROM projects p \
             LEFT JOIN project_progress_claims pc \
               ON pc.project_id = p.id AND pc.organization_id = p.organization_id \
             WHERE p.organization_id = $1 \
             GROUP BY p.id, p.name, p.billing_method, p.budget_amount \
             ORDER BY p.name",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let report_rows: Vec<ProjectBillingRow> = rows
            .iter()
            .map(|r| ProjectBillingRow {
                project_id: r.project_id.to_string(),
                project_name: r.project_name.clone(),
                billing_method: r.billing_method.clone(),
                contract_amount: r.contract_amount,
                billed_amount: r.billed_amount,
                retainage_held: r.retainage_held,
                unbilled_amount: r.contract_amount - r.billed_amount,
            })
            .collect();

        let total_contract = report_rows.iter().map(|r| r.contract_amount).sum();
        let total_billed = report_rows.iter().map(|r| r.billed_amount).sum();
        let total_retainage = report_rows.iter().map(|r| r.retainage_held).sum();
        let total_unbilled = report_rows.iter().map(|r| r.unbilled_amount).sum();

        Ok(ProjectBillingReport {
            rows: report_rows,
            total_contract,
            total_billed,
            total_retainage,
            total_unbilled,
        })
    }
}
