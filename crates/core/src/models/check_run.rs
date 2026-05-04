use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

#[derive(Debug, Serialize)]
pub struct CheckRun {
    pub id: String,
    pub organization_id: String,
    pub bank_account_id: String,
    #[serde(with = "crate::models::date_serde")]
    pub run_date: Date,
    pub status: String,
    pub starting_check_number: Option<i32>,
    pub notes: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct CheckRunItem {
    pub id: String,
    pub check_run_id: String,
    pub payee_id: Option<String>,
    pub payee_name: String,
    pub amount: i64,
    pub memo: Option<String>,
    pub check_number: Option<i32>,
    pub status: String,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateCheckRun {
    pub bank_account_id: String,
    #[serde(with = "crate::models::date_serde")]
    pub run_date: Date,
    pub starting_check_number: Option<i32>,
    pub notes: Option<String>,
    pub items: Vec<CreateCheckRunItem>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCheckRunItem {
    pub payee_id: Option<String>,
    pub payee_name: String,
    pub amount: i64,
    pub memo: Option<String>,
}
