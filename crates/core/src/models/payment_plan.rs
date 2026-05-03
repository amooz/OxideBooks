use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::date_serde;
use crate::money::MinorUnits;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentPlanInstallment {
    pub id: String,
    pub plan_id: String,
    pub due_date: Date,
    pub amount: MinorUnits,
    pub paid_amount: MinorUnits,
    pub status: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentPlan {
    pub id: String,
    pub organization_id: String,
    pub invoice_id: String,
    pub contact_id: String,
    pub description: Option<String>,
    pub total_amount: MinorUnits,
    pub paid_amount: MinorUnits,
    pub remaining_amount: MinorUnits,
    pub status: String,
    pub installments: Vec<PaymentPlanInstallment>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateInstallment {
    #[serde(with = "date_serde")]
    pub due_date: Date,
    pub amount: MinorUnits,
}

#[derive(Debug, Deserialize)]
pub struct CreatePaymentPlan {
    pub invoice_id: String,
    pub description: Option<String>,
    pub installments: Vec<CreateInstallment>,
}

/// Body for POST /payment-plans/:id/installments/:inst_id/pay
#[derive(Debug, Deserialize)]
pub struct PayInstallment {
    pub amount: MinorUnits,
}
