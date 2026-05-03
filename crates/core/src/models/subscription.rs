use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::{date_serde, opt_date_serde};
use crate::money::MinorUnits;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionPlan {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub description: Option<String>,
    pub price: MinorUnits,
    pub currency: String,
    pub billing_cycle: String,
    pub trial_days: i32,
    pub is_active: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub organization_id: String,
    pub plan_id: String,
    pub plan_name: String,
    pub contact_id: String,
    pub status: String,
    pub quantity: i32,
    pub unit_price: MinorUnits,
    pub billing_amount: MinorUnits,
    #[serde(with = "date_serde")]
    pub current_period_start: Date,
    #[serde(with = "date_serde")]
    pub current_period_end: Date,
    #[serde(default, with = "opt_date_serde")]
    pub trial_end: Option<Date>,
    pub cancel_at_period_end: bool,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub cancelled_at: Option<OffsetDateTime>,
    pub notes: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateSubscriptionPlan {
    pub name: String,
    pub description: Option<String>,
    pub price: MinorUnits,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(default = "default_cycle")]
    pub billing_cycle: String,
    #[serde(default)]
    pub trial_days: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSubscriptionPlan {
    pub name: Option<String>,
    pub description: Option<String>,
    pub price: Option<MinorUnits>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSubscription {
    pub plan_id: String,
    pub contact_id: String,
    #[serde(default = "default_quantity")]
    pub quantity: i32,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSubscription {
    pub quantity: Option<i32>,
    pub cancel_at_period_end: Option<bool>,
    pub notes: Option<String>,
}

fn default_currency() -> String {
    "USD".to_string()
}
fn default_cycle() -> String {
    "monthly".to_string()
}
fn default_quantity() -> i32 {
    1
}
