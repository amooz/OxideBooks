use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::date_serde;
use crate::money::MinorUnits;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditNote {
    pub id: String,
    pub organization_id: String,
    pub contact_id: Option<String>,
    #[serde(with = "date_serde")]
    pub note_date: Date,
    pub reference: Option<String>,
    pub description: String,
    pub amount: MinorUnits,
    pub remaining: MinorUnits,
    pub status: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditNoteApplication {
    pub id: String,
    pub credit_note_id: String,
    pub invoice_id: String,
    pub amount_applied: MinorUnits,
    #[serde(with = "time::serde::rfc3339")]
    pub applied_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateCreditNote {
    pub contact_id: Option<String>,
    #[serde(with = "date_serde")]
    pub note_date: Date,
    pub reference: Option<String>,
    #[serde(default)]
    pub description: String,
    pub amount: MinorUnits,
}

#[derive(Debug, Deserialize)]
pub struct ApplyCreditNote {
    pub invoice_id: String,
    pub amount: MinorUnits,
}
