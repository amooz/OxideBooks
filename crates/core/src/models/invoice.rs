use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::{money::MinorUnits, CoreError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceType {
    /// Money owed to us by a customer
    Invoice,
    /// Money we owe to a vendor
    Bill,
}

impl std::fmt::Display for InvoiceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            InvoiceType::Invoice => "invoice",
            InvoiceType::Bill => "bill",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceStatus {
    Draft,
    Sent,
    Partial,
    Paid,
    Overdue,
    Voided,
}

impl std::fmt::Display for InvoiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            InvoiceStatus::Draft => "draft",
            InvoiceStatus::Sent => "sent",
            InvoiceStatus::Partial => "partial",
            InvoiceStatus::Paid => "paid",
            InvoiceStatus::Overdue => "overdue",
            InvoiceStatus::Voided => "voided",
        };
        f.write_str(s)
    }
}

impl std::str::FromStr for InvoiceStatus {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(InvoiceStatus::Draft),
            "sent" => Ok(InvoiceStatus::Sent),
            "partial" => Ok(InvoiceStatus::Partial),
            "paid" => Ok(InvoiceStatus::Paid),
            "overdue" => Ok(InvoiceStatus::Overdue),
            "voided" => Ok(InvoiceStatus::Voided),
            _ => Err(CoreError::UnknownInvoiceStatus(s.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: String,
    pub organization_id: String,
    pub invoice_number: String,
    pub contact_id: String,
    pub invoice_type: InvoiceType,
    pub status: InvoiceStatus,
    #[serde(with = "crate::models::date_serde")]
    pub date: Date,
    #[serde(with = "crate::models::date_serde")]
    pub due_date: Date,
    pub currency: String,
    pub notes: Option<String>,
    pub lines: Vec<InvoiceLine>,
    pub journal_entry_id: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl Invoice {
    /// Subtotal before tax, in minor units.
    pub fn subtotal(&self) -> MinorUnits {
        self.lines.iter().map(|l| l.line_total()).sum()
    }

    /// Tax total, in minor units.
    pub fn tax_total(&self) -> MinorUnits {
        self.lines.iter().map(|l| l.tax_amount()).sum()
    }

    /// Grand total including tax, in minor units.
    pub fn total(&self) -> MinorUnits {
        self.subtotal() + self.tax_total()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceLine {
    pub id: String,
    pub invoice_id: String,
    pub description: String,
    pub account_id: Option<String>,
    /// Quantity × 100 (e.g. 1.5 units → 150)
    pub quantity: i64,
    /// Unit price in minor units
    pub unit_price: MinorUnits,
    /// Tax rate × 100 (e.g. 10% → 1000)
    pub tax_rate: i64,
    pub sort_order: i32,
}

impl InvoiceLine {
    pub fn line_total(&self) -> MinorUnits {
        self.quantity * self.unit_price / 100
    }

    pub fn tax_amount(&self) -> MinorUnits {
        self.line_total() * self.tax_rate / 10_000
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateInvoice {
    pub contact_id: String,
    pub invoice_type: InvoiceType,
    #[serde(with = "crate::models::date_serde")]
    pub date: Date,
    #[serde(with = "crate::models::date_serde")]
    pub due_date: Date,
    pub currency: Option<String>,
    pub notes: Option<String>,
    pub lines: Vec<CreateInvoiceLine>,
}

impl CreateInvoice {
    pub fn validate(&self) -> Result<(), CoreError> {
        for line in &self.lines {
            if line.quantity <= 0 {
                return Err(CoreError::ZeroQuantity);
            }
            if line.unit_price < 0 {
                return Err(CoreError::NegativeAmount(line.unit_price));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateInvoiceLine {
    pub description: String,
    pub account_id: Option<String>,
    /// Quantity × 100
    pub quantity: i64,
    pub unit_price: MinorUnits,
    /// Tax rate × 100 (e.g. 10% → 1000); defaults to 0
    pub tax_rate: Option<i64>,
}
