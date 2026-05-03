use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::opt_date_serde;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Frequency {
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

impl std::fmt::Display for Frequency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Frequency::Weekly => "weekly",
            Frequency::Monthly => "monthly",
            Frequency::Quarterly => "quarterly",
            Frequency::Yearly => "yearly",
        })
    }
}

impl std::str::FromStr for Frequency {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "weekly" => Ok(Self::Weekly),
            "monthly" => Ok(Self::Monthly),
            "quarterly" => Ok(Self::Quarterly),
            "yearly" => Ok(Self::Yearly),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurringSchedule {
    pub id: String,
    pub organization_id: String,
    pub template: serde_json::Value,
    pub frequency: Frequency,
    pub interval_count: i32,
    #[serde(with = "crate::models::date_serde")]
    pub next_due_date: Date,
    #[serde(default, with = "opt_date_serde")]
    pub end_date: Option<Date>,
    pub auto_send: bool,
    pub is_active: bool,
    pub max_occurrences: Option<i32>,
    pub occurrences_count: i32,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRecurringSchedule {
    pub template: serde_json::Value,
    pub frequency: Frequency,
    #[serde(default = "one")]
    pub interval_count: i32,
    #[serde(with = "crate::models::date_serde")]
    pub next_due_date: Date,
    #[serde(default, with = "opt_date_serde")]
    pub end_date: Option<Date>,
    #[serde(default)]
    pub auto_send: bool,
}

fn one() -> i32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateRecurringSchedule {
    #[serde(default, with = "opt_date_serde")]
    pub next_due_date: Option<Date>,
    #[serde(default, with = "opt_date_serde")]
    pub end_date: Option<Date>,
    pub auto_send: Option<bool>,
    pub is_active: Option<bool>,
    pub max_occurrences: Option<i32>,
}
