use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::opt_date_serde;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DepreciationMethod {
    StraightLine,
    DecliningBalance,
}

impl std::fmt::Display for DepreciationMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            DepreciationMethod::StraightLine => "straight_line",
            DepreciationMethod::DecliningBalance => "declining_balance",
        })
    }
}

impl std::str::FromStr for DepreciationMethod {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "straight_line" => Ok(Self::StraightLine),
            "declining_balance" => Ok(Self::DecliningBalance),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixedAsset {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub asset_number: String,
    #[serde(with = "crate::models::date_serde")]
    pub purchase_date: Date,
    pub purchase_cost: i64,
    pub salvage_value: i64,
    pub useful_life_months: i32,
    pub depreciation_method: DepreciationMethod,
    pub asset_account_id: Option<String>,
    pub accumulated_depreciation_acct: Option<String>,
    pub depreciation_expense_acct: Option<String>,
    pub status: String,
    #[serde(default, with = "opt_date_serde")]
    pub disposed_at: Option<Date>,
    pub total_depreciated: i64,
    pub book_value: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFixedAsset {
    pub name: String,
    pub asset_number: String,
    #[serde(with = "crate::models::date_serde")]
    pub purchase_date: Date,
    pub purchase_cost: i64,
    #[serde(default)]
    pub salvage_value: i64,
    pub useful_life_months: i32,
    #[serde(default = "default_method")]
    pub depreciation_method: DepreciationMethod,
    pub asset_account_id: Option<String>,
    pub accumulated_depreciation_acct: Option<String>,
    pub depreciation_expense_acct: Option<String>,
}

fn default_method() -> DepreciationMethod {
    DepreciationMethod::StraightLine
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateFixedAsset {
    pub name: Option<String>,
    pub asset_account_id: Option<String>,
    pub accumulated_depreciation_acct: Option<String>,
    pub depreciation_expense_acct: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRegisterRow {
    pub id: String,
    pub name: String,
    pub asset_number: String,
    pub purchase_cost: i64,
    pub salvage_value: i64,
    pub total_depreciated: i64,
    pub book_value: i64,
    pub status: String,
}
