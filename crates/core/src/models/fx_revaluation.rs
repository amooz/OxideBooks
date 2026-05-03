use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::date_serde;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxRevaluation {
    pub id: String,
    pub organization_id: String,
    #[serde(with = "date_serde")]
    pub revaluation_date: Date,
    pub currency: String,
    pub rate: Decimal,
    pub net_gain_loss: i64,
    pub journal_entry_id: Option<String>,
    pub notes: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateFxRevaluation {
    #[serde(with = "date_serde")]
    pub revaluation_date: Date,
    pub currency: String,
    pub rate: Decimal,
    pub notes: Option<String>,
}
