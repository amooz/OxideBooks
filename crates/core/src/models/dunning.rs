use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DunningRule {
    pub id: String,
    pub organization_id: String,
    pub days_overdue: i32,
    pub reminder_level: i32,
    pub subject_template: String,
    pub body_template: String,
    pub is_active: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDunningRule {
    pub days_overdue: i32,
    #[serde(default = "default_level")]
    pub reminder_level: i32,
    #[serde(default = "default_subject")]
    pub subject_template: String,
    #[serde(default = "default_body")]
    pub body_template: String,
}

fn default_level() -> i32 {
    1
}
fn default_subject() -> String {
    "Invoice overdue reminder".to_string()
}
fn default_body() -> String {
    "Your invoice is overdue. Please arrange payment at your earliest convenience.".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceReminder {
    pub id: String,
    pub invoice_id: String,
    pub rule_id: Option<String>,
    pub to_address: String,
    pub level: i32,
    #[serde(with = "time::serde::rfc3339")]
    pub sent_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverdueInvoice {
    pub invoice_id: String,
    pub invoice_number: String,
    pub contact_id: String,
    pub days_overdue: i64,
    pub amount_due: i64,
    pub currency: String,
    pub last_reminder_level: Option<i32>,
}
