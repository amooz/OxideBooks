use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::money::MinorUnits;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentTerms {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub net_days: i32,
    pub discount_days: Option<i32>,
    /// Early-payment discount percent × 100 (e.g. 2% → 200); 0 = no discount
    pub discount_pct: MinorUnits,
    pub is_default: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreatePaymentTerms {
    pub name: String,
    pub net_days: i32,
    pub discount_days: Option<i32>,
    #[serde(default)]
    pub discount_pct: MinorUnits,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePaymentTerms {
    pub name: Option<String>,
    pub net_days: Option<i32>,
    pub discount_days: Option<i32>,
    pub discount_pct: Option<MinorUnits>,
    pub is_default: Option<bool>,
}
