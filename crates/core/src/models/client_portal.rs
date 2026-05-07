use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientPortalToken {
    pub id: String,
    pub token: String,
    pub contact_id: String,
    pub organization_id: String,
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateClientPortalToken {
    pub contact_id: String,
    #[serde(default = "default_expiry_hours")]
    pub expires_hours: i64,
}

fn default_expiry_hours() -> i64 {
    72
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalPaymentMethod {
    pub id: String,
    pub organization_id: String,
    pub contact_id: String,
    pub payment_type: String,
    pub provider: String,
    pub provider_token: String,
    pub last4: Option<String>,
    pub brand: Option<String>,
    pub exp_month: Option<i16>,
    pub exp_year: Option<i16>,
    pub bank_name: Option<String>,
    pub is_default: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePortalPaymentMethod {
    pub payment_type: String,
    pub provider_token: String,
    pub last4: Option<String>,
    pub brand: Option<String>,
    pub exp_month: Option<i16>,
    pub exp_year: Option<i16>,
    pub bank_name: Option<String>,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalAutopayEnrollment {
    pub id: String,
    pub organization_id: String,
    pub contact_id: String,
    pub payment_method_id: String,
    pub is_active: bool,
    pub days_before_due: i32,
    pub max_amount: Option<i64>,
    #[serde(with = "time::serde::rfc3339")]
    pub enrolled_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub cancelled_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePortalAutopay {
    pub payment_method_id: String,
    #[serde(default)]
    pub days_before_due: i32,
    pub max_amount: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalPayInvoice {
    pub payment_method_id: String,
}
