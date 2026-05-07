use oxidebooks_core::models::{
    HstGstReturn, T4AFilingSummary, T4ASlip, T4Slip, T4Summary, TaxFiling,
};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct FilingRow {
    id: Uuid,
    organization_id: Uuid,
    filing_type: String,
    period_year: i32,
    period_quarter: Option<i32>,
    period_from: Option<Date>,
    period_to: Option<Date>,
    tax_jurisdiction: String,
    status: String,
    summary_data: Option<serde_json::Value>,
    efile_xml: Option<String>,
    submitted_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn from_row(r: FilingRow) -> TaxFiling {
    TaxFiling {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        filing_type: r.filing_type,
        period_year: r.period_year,
        period_quarter: r.period_quarter,
        period_from: r.period_from,
        period_to: r.period_to,
        tax_jurisdiction: r.tax_jurisdiction,
        status: r.status,
        summary_data: r.summary_data,
        efile_xml: r.efile_xml,
        submitted_at: r.submitted_at,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

const COLS: &str =
    "id, organization_id, filing_type, period_year, period_quarter, period_from, period_to, \
     tax_jurisdiction, status, summary_data, efile_xml, submitted_at, created_at, updated_at";

pub struct TaxFilingRepo;

impl TaxFilingRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<TaxFiling>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<FilingRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM tax_filings WHERE organization_id = $1 \
             ORDER BY period_year DESC, created_at DESC"
        ))
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<TaxFiling, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: FilingRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM tax_filings WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(from_row(row))
    }

    pub async fn submit(pool: &PgPool, org_id: &str, id: &str) -> Result<TaxFiling, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n = sqlx::query(
            "UPDATE tax_filings SET status = 'submitted', submitted_at = NOW(), updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2 AND status = 'draft'",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::Conflict(
                "filing not found or not in draft status".into(),
            ));
        }
        Self::get_by_id(pool, org_id, id).await
    }

    /// Generate 1099-NEC filing records for contractors paid ≥ $600 in the given year.
    pub async fn generate_1099s(
        pool: &PgPool,
        org_id: &str,
        year: i32,
    ) -> Result<TaxFiling, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        #[derive(sqlx::FromRow)]
        struct Row {
            contact_id: Uuid,
            contact_name: String,
            tax_id: Option<String>,
            total_paid: i64,
        }

        // $600 threshold = 60000 minor units
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT c.id AS contact_id, c.name AS contact_name, c.tax_id, \
             COALESCE(SUM(p.amount), 0)::BIGINT AS total_paid \
             FROM contacts c \
             LEFT JOIN invoices i ON i.contact_id = c.id \
               AND i.organization_id = c.organization_id \
               AND i.invoice_type = 'bill' \
             LEFT JOIN payments p ON p.invoice_id = i.id \
               AND EXTRACT(YEAR FROM p.payment_date) = $2 \
             WHERE c.organization_id = $1 AND c.is_1099_vendor = TRUE \
             GROUP BY c.id, c.name, c.tax_id \
             HAVING COALESCE(SUM(p.amount), 0) >= 60000 \
             ORDER BY c.name",
        )
        .bind(org_uuid)
        .bind(year)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let slips: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "contact_id": r.contact_id.to_string(),
                    "contact_name": r.contact_name,
                    "tax_id": r.tax_id,
                    "total_paid": r.total_paid,
                })
            })
            .collect();

        let total: i64 = rows.iter().map(|r| r.total_paid).sum();
        let summary = serde_json::json!({
            "year": year,
            "slips": slips,
            "total_paid": total,
            "count": rows.len(),
        });

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO tax_filings \
             (organization_id, filing_type, period_year, tax_jurisdiction, summary_data) \
             VALUES ($1, '1099_nec', $2, 'us_federal', $3) RETURNING id",
        )
        .bind(org_uuid)
        .bind(year)
        .bind(&summary)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    /// Generate a 941 (quarterly payroll tax) filing for the given year/quarter.
    pub async fn generate_941(
        pool: &PgPool,
        org_id: &str,
        year: i32,
        quarter: i32,
    ) -> Result<TaxFiling, DbError> {
        if !(1..=4).contains(&quarter) {
            return Err(DbError::Conflict("quarter must be 1–4".into()));
        }
        let org_uuid = parse_uuid(org_id)?;

        // Quarter month ranges: Q1=1-3, Q2=4-6, Q3=7-9, Q4=10-12
        let month_start = (quarter - 1) * 3 + 1;
        let month_end = quarter * 3;

        #[derive(sqlx::FromRow)]
        struct Row {
            total_wages: i64,
            total_tax_withheld: i64,
            employee_count: i64,
        }

        let row: Row = sqlx::query_as(
            "SELECT \
               COALESCE(SUM(ps.gross_pay), 0)::BIGINT AS total_wages, \
               COALESCE(SUM(ps.tax_withheld), 0)::BIGINT AS total_tax_withheld, \
               COUNT(DISTINCT ps.employee_id)::BIGINT AS employee_count \
             FROM payslips ps \
             JOIN payroll_runs pr ON pr.id = ps.payroll_run_id \
             WHERE pr.organization_id = $1 \
               AND EXTRACT(YEAR FROM pr.pay_date) = $2 \
               AND EXTRACT(MONTH FROM pr.pay_date) BETWEEN $3 AND $4",
        )
        .bind(org_uuid)
        .bind(year)
        .bind(month_start)
        .bind(month_end)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        // Social Security: 6.2% employee + 6.2% employer = 12.4% of wages (up to limit)
        let ss_tax = row.total_wages * 124 / 1000;
        // Medicare: 1.45% employee + 1.45% employer = 2.9%
        let medicare_tax = row.total_wages * 29 / 1000;

        let summary = serde_json::json!({
            "year": year,
            "quarter": quarter,
            "total_wages": row.total_wages,
            "federal_income_tax_withheld": row.total_tax_withheld,
            "social_security_tax": ss_tax,
            "medicare_tax": medicare_tax,
            "total_taxes": row.total_tax_withheld + ss_tax + medicare_tax,
            "employee_count": row.employee_count,
        });

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO tax_filings \
             (organization_id, filing_type, period_year, period_quarter, \
              tax_jurisdiction, summary_data) \
             VALUES ($1, '941', $2, $3, 'us_federal', $4) RETURNING id",
        )
        .bind(org_uuid)
        .bind(year)
        .bind(quarter)
        .bind(&summary)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    /// Generate T4 slips from payroll data and return a filing record with CRA XML.
    pub async fn generate_t4s(
        pool: &PgPool,
        org_id: &str,
        year: i32,
    ) -> Result<TaxFiling, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        #[derive(sqlx::FromRow)]
        struct PayRow {
            employee_id: Uuid,
            employee_name: String,
            gross_pay: i64,
            income_tax_deducted: i64,
        }

        let rows: Vec<PayRow> = sqlx::query_as(
            "SELECT \
               e.id AS employee_id, \
               (e.first_name || ' ' || e.last_name) AS employee_name, \
               COALESCE(SUM(ps.gross_pay), 0)::BIGINT AS gross_pay, \
               COALESCE(SUM(ps.tax_withheld), 0)::BIGINT AS income_tax_deducted \
             FROM employees e \
             JOIN payslips ps ON ps.employee_id = e.id \
             JOIN payroll_runs pr ON pr.id = ps.payroll_run_id \
             WHERE pr.organization_id = $1 \
               AND EXTRACT(YEAR FROM pr.pay_date) = $2 \
             GROUP BY e.id, e.first_name, e.last_name \
             ORDER BY e.last_name, e.first_name",
        )
        .bind(org_uuid)
        .bind(year)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        // CPP employee rate 2024: 5.95% of pensionable earnings
        // EI employee rate 2024: 1.66% of insurable earnings
        let slips: Vec<T4Slip> = rows
            .iter()
            .map(|r| {
                let cpp = r.gross_pay * 595 / 10_000;
                let ei = r.gross_pay * 166 / 10_000;
                T4Slip {
                    employee_id: r.employee_id.to_string(),
                    employee_name: r.employee_name.clone(),
                    sin: None,
                    employment_income: r.gross_pay,
                    income_tax_deducted: r.income_tax_deducted,
                    cpp_employee: cpp,
                    ei_employee: ei,
                }
            })
            .collect();

        let total_income = slips.iter().map(|s| s.employment_income).sum::<i64>();
        let total_tax = slips.iter().map(|s| s.income_tax_deducted).sum::<i64>();
        let total_cpp = slips.iter().map(|s| s.cpp_employee).sum::<i64>();
        let total_ei = slips.iter().map(|s| s.ei_employee).sum::<i64>();

        let summary_data = serde_json::json!({
            "year": year,
            "slips": slips,
            "total_employment_income": total_income,
            "total_income_tax_deducted": total_tax,
            "total_cpp_employee": total_cpp,
            "total_ei_employee": total_ei,
            "total_cpp_employer": total_cpp,  // employer matches employee
            "total_ei_employer": total_ei * 140 / 100,  // employer = employee × 1.4
        });

        // Minimal CRA T4 XML (T4SUM envelope)
        let efile_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<T4Slip xmlns="http://www.cra-arc.gc.ca/xmlns/t4/2023" year="{year}">
  <T4Summary>
    <TaxYear>{year}</TaxYear>
    <TotalEmploymentIncome>{total_income}</TotalEmploymentIncome>
    <TotalIncomeTaxDeducted>{total_tax}</TotalIncomeTaxDeducted>
    <TotalCPPContributions>{total_cpp}</TotalCPPContributions>
    <TotalEIPremiums>{total_ei}</TotalEIPremiums>
    <SlipCount>{}</SlipCount>
  </T4Summary>
</T4Slip>"#,
            slips.len()
        );

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO tax_filings \
             (organization_id, filing_type, period_year, tax_jurisdiction, \
              summary_data, efile_xml) \
             VALUES ($1, 't4', $2, 'ca_federal', $3, $4) RETURNING id",
        )
        .bind(org_uuid)
        .bind(year)
        .bind(&summary_data)
        .bind(&efile_xml)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    /// Generate T4A slips for self-employed/contractor payments ≥ $500 CAD.
    pub async fn generate_t4a(
        pool: &PgPool,
        org_id: &str,
        year: i32,
    ) -> Result<TaxFiling, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        // $500 CAD = 50000 minor units
        #[derive(sqlx::FromRow)]
        struct Row {
            contact_id: Uuid,
            contact_name: String,
            tax_id: Option<String>,
            total_paid: i64,
        }

        let rows: Vec<Row> = sqlx::query_as(
            "SELECT c.id AS contact_id, c.name AS contact_name, c.tax_id, \
             COALESCE(SUM(p.amount), 0)::BIGINT AS total_paid \
             FROM contacts c \
             LEFT JOIN invoices i ON i.contact_id = c.id \
               AND i.organization_id = c.organization_id \
               AND i.invoice_type = 'bill' \
             LEFT JOIN payments p ON p.invoice_id = i.id \
               AND EXTRACT(YEAR FROM p.payment_date) = $2 \
             WHERE c.organization_id = $1 AND c.is_1099_vendor = TRUE \
             GROUP BY c.id, c.name, c.tax_id \
             HAVING COALESCE(SUM(p.amount), 0) >= 50000 \
             ORDER BY c.name",
        )
        .bind(org_uuid)
        .bind(year)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let slips: Vec<T4ASlip> = rows
            .iter()
            .map(|r| T4ASlip {
                contact_id: r.contact_id.to_string(),
                contact_name: r.contact_name.clone(),
                sin: r.tax_id.clone(),
                fees_for_services: r.total_paid,
            })
            .collect();

        let total: i64 = slips.iter().map(|s| s.fees_for_services).sum();

        let summary_data = serde_json::json!({
            "year": year,
            "slips": slips,
            "total_fees_for_services": total,
        });

        let efile_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<T4ASlips xmlns="http://www.cra-arc.gc.ca/xmlns/t4a/2023" year="{year}">
  <T4ASummary>
    <TaxYear>{year}</TaxYear>
    <TotalFeesForServices>{total}</TotalFeesForServices>
    <SlipCount>{}</SlipCount>
  </T4ASummary>
</T4ASlips>"#,
            slips.len()
        );

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO tax_filings \
             (organization_id, filing_type, period_year, tax_jurisdiction, \
              summary_data, efile_xml) \
             VALUES ($1, 't4a', $2, 'ca_federal', $3, $4) RETURNING id",
        )
        .bind(org_uuid)
        .bind(year)
        .bind(&summary_data)
        .bind(&efile_xml)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    /// Compute and save a GST/HST return for the given period.
    pub async fn generate_hst_return(
        pool: &PgPool,
        org_id: &str,
        from: Date,
        to: Date,
    ) -> Result<TaxFiling, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        // GST/HST collected: sum of invoice line tax amounts in period
        let gst_collected: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(il.quantity * il.unit_price / 100 * il.tax_rate / 10000), 0)::BIGINT \
             FROM invoices inv \
             JOIN invoice_lines il ON il.invoice_id = inv.id \
             WHERE inv.organization_id = $1 \
               AND inv.invoice_type = 'invoice' \
               AND inv.status NOT IN ('draft', 'voided') \
               AND inv.date BETWEEN $2 AND $3 \
               AND il.tax_rate > 0",
        )
        .bind(org_uuid)
        .bind(from)
        .bind(to)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        // ITC: tax paid on bills (input tax credits)
        let input_tax_credits: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(bl.quantity * bl.unit_price / 100 * bl.tax_rate / 10000), 0)::BIGINT \
             FROM vendor_bills b \
             JOIN bill_lines bl ON bl.bill_id = b.id \
             WHERE b.organization_id = $1 \
               AND b.status NOT IN ('draft', 'voided') \
               AND b.bill_date BETWEEN $2 AND $3 \
               AND bl.tax_rate > 0",
        )
        .bind(org_uuid)
        .bind(from)
        .bind(to)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        // Total revenue (Line 101)
        let total_revenue: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(il.quantity * il.unit_price / 100), 0)::BIGINT \
             FROM invoices inv \
             JOIN invoice_lines il ON il.invoice_id = inv.id \
             WHERE inv.organization_id = $1 \
               AND inv.invoice_type = 'invoice' \
               AND inv.status NOT IN ('draft', 'voided') \
               AND inv.date BETWEEN $2 AND $3",
        )
        .bind(org_uuid)
        .bind(from)
        .bind(to)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let net_tax = gst_collected - input_tax_credits;
        let year = from.year();

        let summary_data = serde_json::json!({
            "from": from.to_string(),
            "to": to.to_string(),
            "total_revenue": total_revenue,
            "gst_collected": gst_collected,
            "input_tax_credits": input_tax_credits,
            "net_tax": net_tax,
        });

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO tax_filings \
             (organization_id, filing_type, period_year, period_from, period_to, \
              tax_jurisdiction, summary_data) \
             VALUES ($1, 'hst_gst', $2, $3, $4, 'ca_federal', $5) RETURNING id",
        )
        .bind(org_uuid)
        .bind(year)
        .bind(from)
        .bind(to)
        .bind(&summary_data)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    /// Build T4Summary from payroll data (report, does not persist).
    pub async fn t4_summary(pool: &PgPool, org_id: &str, year: i32) -> Result<T4Summary, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        #[derive(sqlx::FromRow)]
        struct PayRow {
            employee_id: Uuid,
            employee_name: String,
            gross_pay: i64,
            income_tax_deducted: i64,
        }

        let rows: Vec<PayRow> = sqlx::query_as(
            "SELECT \
               e.id AS employee_id, \
               (e.first_name || ' ' || e.last_name) AS employee_name, \
               COALESCE(SUM(ps.gross_pay), 0)::BIGINT AS gross_pay, \
               COALESCE(SUM(ps.tax_withheld), 0)::BIGINT AS income_tax_deducted \
             FROM employees e \
             JOIN payslips ps ON ps.employee_id = e.id \
             JOIN payroll_runs pr ON pr.id = ps.payroll_run_id \
             WHERE pr.organization_id = $1 \
               AND EXTRACT(YEAR FROM pr.pay_date) = $2 \
             GROUP BY e.id, e.first_name, e.last_name \
             ORDER BY e.last_name, e.first_name",
        )
        .bind(org_uuid)
        .bind(year)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let slips: Vec<T4Slip> = rows
            .iter()
            .map(|r| {
                let cpp = r.gross_pay * 595 / 10_000;
                let ei = r.gross_pay * 166 / 10_000;
                T4Slip {
                    employee_id: r.employee_id.to_string(),
                    employee_name: r.employee_name.clone(),
                    sin: None,
                    employment_income: r.gross_pay,
                    income_tax_deducted: r.income_tax_deducted,
                    cpp_employee: cpp,
                    ei_employee: ei,
                }
            })
            .collect();

        let total_employment_income = slips.iter().map(|s| s.employment_income).sum();
        let total_income_tax_deducted = slips.iter().map(|s| s.income_tax_deducted).sum();
        let total_cpp_employee = slips.iter().map(|s| s.cpp_employee).sum();
        let total_ei_employee = slips.iter().map(|s| s.ei_employee).sum::<i64>();

        Ok(T4Summary {
            year,
            slips,
            total_employment_income,
            total_income_tax_deducted,
            total_cpp_employee,
            total_ei_employee,
            total_cpp_employer: total_cpp_employee,
            total_ei_employer: total_ei_employee * 140 / 100,
        })
    }

    /// Compute HST/GST return without persisting (report only).
    pub async fn hst_gst_return(
        pool: &PgPool,
        org_id: &str,
        from: Date,
        to: Date,
    ) -> Result<HstGstReturn, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let gst_collected: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(il.quantity * il.unit_price / 100 * il.tax_rate / 10000), 0)::BIGINT \
             FROM invoices inv \
             JOIN invoice_lines il ON il.invoice_id = inv.id \
             WHERE inv.organization_id = $1 \
               AND inv.invoice_type = 'invoice' \
               AND inv.status NOT IN ('draft', 'voided') \
               AND inv.date BETWEEN $2 AND $3 \
               AND il.tax_rate > 0",
        )
        .bind(org_uuid)
        .bind(from)
        .bind(to)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let input_tax_credits: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(bl.quantity * bl.unit_price / 100 * bl.tax_rate / 10000), 0)::BIGINT \
             FROM vendor_bills b \
             JOIN bill_lines bl ON bl.bill_id = b.id \
             WHERE b.organization_id = $1 \
               AND b.status NOT IN ('draft', 'voided') \
               AND b.bill_date BETWEEN $2 AND $3 \
               AND bl.tax_rate > 0",
        )
        .bind(org_uuid)
        .bind(from)
        .bind(to)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let total_revenue: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(il.quantity * il.unit_price / 100), 0)::BIGINT \
             FROM invoices inv \
             JOIN invoice_lines il ON il.invoice_id = inv.id \
             WHERE inv.organization_id = $1 \
               AND inv.invoice_type = 'invoice' \
               AND inv.status NOT IN ('draft', 'voided') \
               AND inv.date BETWEEN $2 AND $3",
        )
        .bind(org_uuid)
        .bind(from)
        .bind(to)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(HstGstReturn {
            from,
            to,
            total_revenue,
            gst_collected,
            input_tax_credits,
            net_tax: gst_collected - input_tax_credits,
        })
    }

    /// T4A summary (report only, does not persist).
    pub async fn t4a_summary(
        pool: &PgPool,
        org_id: &str,
        year: i32,
    ) -> Result<T4AFilingSummary, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        #[derive(sqlx::FromRow)]
        struct Row {
            contact_id: Uuid,
            contact_name: String,
            tax_id: Option<String>,
            total_paid: i64,
        }

        let rows: Vec<Row> = sqlx::query_as(
            "SELECT c.id AS contact_id, c.name AS contact_name, c.tax_id, \
             COALESCE(SUM(p.amount), 0)::BIGINT AS total_paid \
             FROM contacts c \
             LEFT JOIN invoices i ON i.contact_id = c.id \
               AND i.organization_id = c.organization_id \
               AND i.invoice_type = 'bill' \
             LEFT JOIN payments p ON p.invoice_id = i.id \
               AND EXTRACT(YEAR FROM p.payment_date) = $2 \
             WHERE c.organization_id = $1 AND c.is_1099_vendor = TRUE \
             GROUP BY c.id, c.name, c.tax_id \
             HAVING COALESCE(SUM(p.amount), 0) >= 50000 \
             ORDER BY c.name",
        )
        .bind(org_uuid)
        .bind(year)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let slips: Vec<T4ASlip> = rows
            .iter()
            .map(|r| T4ASlip {
                contact_id: r.contact_id.to_string(),
                contact_name: r.contact_name.clone(),
                sin: r.tax_id.clone(),
                fees_for_services: r.total_paid,
            })
            .collect();

        let total_fees_for_services = slips.iter().map(|s| s.fees_for_services).sum();

        Ok(T4AFilingSummary {
            year,
            slips,
            total_fees_for_services,
        })
    }
}
