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

#[derive(Debug, Deserialize)]
pub struct ChangePlan {
    pub new_plan_id: String,
    /// If true, issue a prorated credit note for unused days on the old plan.
    #[serde(default)]
    pub prorate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanChange {
    pub id: String,
    pub subscription_id: String,
    pub old_plan_id: String,
    pub new_plan_id: String,
    pub old_price: MinorUnits,
    pub new_price: MinorUnits,
    pub proration_credit: MinorUnits,
    #[serde(with = "time::serde::rfc3339")]
    pub changed_at: OffsetDateTime,
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

/// MRR snapshot at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrrSnapshot {
    #[serde(with = "date_serde")]
    pub as_of: Date,
    /// Monthly Recurring Revenue (minor units, base currency).
    pub mrr: MinorUnits,
    /// Annual Recurring Revenue = MRR × 12.
    pub arr: MinorUnits,
    /// Number of active subscriptions contributing to MRR.
    pub active_subscriptions: i64,
    /// Breakdown by plan.
    pub by_plan: Vec<MrrByPlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrrByPlan {
    pub plan_id: String,
    pub plan_name: String,
    pub billing_cycle: String,
    pub active_count: i64,
    pub mrr: MinorUnits,
}

/// Subscription churn analysis over a date range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChurnReport {
    #[serde(with = "date_serde")]
    pub from: Date,
    #[serde(with = "date_serde")]
    pub to: Date,
    pub active_at_start: i64,
    pub new_subscriptions: i64,
    pub churned: i64,
    /// Churned / (active_at_start + new). None if denominator is 0.
    pub churn_rate_pct: Option<f64>,
    pub net_new: i64,
    pub churned_mrr: MinorUnits,
    pub new_mrr: MinorUnits,
    pub net_mrr_change: MinorUnits,
}
