use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AchPayment {
    pub id: String,
    pub organization_id: String,
    pub entry_type: String,
    pub invoice_id: Option<String>,
    pub bill_id: Option<String>,
    pub routing_number: String,
    pub account_number: String,
    pub account_type: String,
    pub amount: i64,
    pub status: String,
    pub trace_number: Option<String>,
    pub effective_date: Date,
    pub return_code: Option<String>,
    pub submitted_at: Option<OffsetDateTime>,
    pub settled_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CollectAch {
    pub routing_number: String,
    pub account_number: String,
    #[serde(default = "default_checking")]
    pub account_type: String,
    /// Effective date in YYYY-MM-DD; defaults to next business day
    pub effective_date: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PayBillAch {
    pub routing_number: String,
    pub account_number: String,
    #[serde(default = "default_checking")]
    pub account_type: String,
    pub effective_date: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateNachaRequest {
    /// Optional: filter to specific effective date (YYYY-MM-DD)
    pub effective_date: Option<String>,
    /// Company name for NACHA file header (max 16 chars)
    pub company_name: Option<String>,
    /// Company ID (EIN/TIN) for NACHA header (max 10 chars)
    pub company_id: Option<String>,
    /// Originating DFI routing number (9 digits)
    pub originating_routing: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NachaFile {
    pub nacha_text: String,
    pub entry_count: usize,
    pub total_debit: i64,
    pub total_credit: i64,
    pub payment_ids: Vec<String>,
}

fn default_checking() -> String {
    "checking".to_string()
}
