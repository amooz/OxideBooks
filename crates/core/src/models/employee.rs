use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::opt_date_serde;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Employee {
    pub id: String,
    pub organization_id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
    pub employee_number: Option<String>,
    #[serde(with = "crate::models::date_serde")]
    pub start_date: Date,
    #[serde(default, with = "opt_date_serde")]
    pub terminated_at: Option<Date>,
    pub pay_type: String,
    pub pay_rate: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateEmployee {
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
    pub employee_number: Option<String>,
    #[serde(with = "crate::models::date_serde")]
    pub start_date: Date,
    pub pay_type: String,
    pub pay_rate: i64,
}

#[derive(Debug, Default, Deserialize)]
pub struct UpdateEmployee {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub employee_number: Option<String>,
    pub pay_type: Option<String>,
    pub pay_rate: Option<i64>,
    #[serde(default, with = "opt_date_serde")]
    pub terminated_at: Option<Date>,
}
