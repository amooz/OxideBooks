use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxFiling {
    pub id: String,
    pub organization_id: String,
    /// One of: 1099_nec | 1099_misc | w2 | 941 | t4 | t4a | hst_gst
    pub filing_type: String,
    pub period_year: i32,
    pub period_quarter: Option<i32>,
    pub period_from: Option<Date>,
    pub period_to: Option<Date>,
    /// us_federal | us_state | ca_federal | ca_provincial
    pub tax_jurisdiction: String,
    pub status: String,
    pub summary_data: Option<serde_json::Value>,
    pub efile_xml: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub submitted_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

// ── T4 / T4A (Canadian payroll slips) ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct T4Slip {
    pub employee_id: String,
    pub employee_name: String,
    pub sin: Option<String>,
    /// Box 14: Employment income
    pub employment_income: i64,
    /// Box 22: Income tax deducted
    pub income_tax_deducted: i64,
    /// Box 16: Employee CPP contributions
    pub cpp_employee: i64,
    /// Box 18: Employee EI premiums
    pub ei_employee: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct T4Summary {
    pub year: i32,
    pub slips: Vec<T4Slip>,
    pub total_employment_income: i64,
    pub total_income_tax_deducted: i64,
    pub total_cpp_employee: i64,
    pub total_ei_employee: i64,
    pub total_cpp_employer: i64,
    pub total_ei_employer: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct T4ASlip {
    pub contact_id: String,
    pub contact_name: String,
    pub sin: Option<String>,
    /// Box 048: Fees for services (self-employed / contractor)
    pub fees_for_services: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct T4AFilingSummary {
    pub year: i32,
    pub slips: Vec<T4ASlip>,
    pub total_fees_for_services: i64,
}

// ── GST / HST Return ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HstGstReturn {
    pub from: Date,
    pub to: Date,
    /// Line 101: Total sales and other revenue
    pub total_revenue: i64,
    /// Line 103: GST/HST collected or collectible
    pub gst_collected: i64,
    /// Line 106: Input tax credits (ITC)
    pub input_tax_credits: i64,
    /// Line 109: Net tax (collected - ITC)
    pub net_tax: i64,
}
