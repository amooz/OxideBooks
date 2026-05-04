use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Serialize)]
pub struct EmployeeBankAccount {
    pub id: String,
    pub organization_id: String,
    pub employee_id: String,
    pub bank_name: String,
    pub routing_number: String,
    pub account_last4: String,
    pub account_type: String,
    pub is_primary: bool,
    pub is_active: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateEmployeeBankAccount {
    pub bank_name: String,
    pub routing_number: String,
    /// Full account number — stored only as last 4 digits.
    pub account_number: String,
    #[serde(default = "default_account_type")]
    pub account_type: String,
    #[serde(default)]
    pub is_primary: bool,
}

fn default_account_type() -> String {
    "checking".into()
}

#[derive(Debug, Deserialize)]
pub struct UpdateEmployeeBankAccount {
    pub bank_name: Option<String>,
    pub is_primary: Option<bool>,
    pub is_active: Option<bool>,
}
