use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::date_serde;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MileageTrip {
    pub id: String,
    pub organization_id: String,
    pub user_id: String,
    #[serde(with = "date_serde")]
    pub trip_date: Date,
    pub distance_km: f64,
    pub purpose: String,
    pub rate_per_km: i64,
    pub reimbursable: bool,
    pub expense_id: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl MileageTrip {
    pub fn reimbursable_amount(&self) -> i64 {
        (self.distance_km * self.rate_per_km as f64).round() as i64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMileageTrip {
    #[serde(with = "date_serde")]
    pub trip_date: Date,
    pub distance_km: f64,
    pub purpose: String,
    #[serde(default)]
    pub rate_per_km: i64,
    #[serde(default = "default_reimbursable")]
    pub reimbursable: bool,
}

fn default_reimbursable() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MileageSummary {
    pub total_km: f64,
    pub total_reimbursable: i64,
    pub trip_count: i64,
}
