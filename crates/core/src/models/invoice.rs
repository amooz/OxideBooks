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
pub struct UpdateInvoice {
    pub status: Option<InvoiceStatus>,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub due_date: Option<Date>,
    pub notes: Option<String>,
}

impl InvoiceStatus {
    /// Returns the set of valid next states from the current status.
    pub fn allowed_transitions(&self) -> &'static [InvoiceStatus] {
        match self {
            InvoiceStatus::Draft => &[InvoiceStatus::Sent, InvoiceStatus::Voided],
            InvoiceStatus::Sent => &[
                InvoiceStatus::Partial,
                InvoiceStatus::Paid,
                InvoiceStatus::Overdue,
                InvoiceStatus::Voided,
            ],
            InvoiceStatus::Partial => &[
                InvoiceStatus::Paid,
                InvoiceStatus::Overdue,
                InvoiceStatus::Voided,
            ],
            InvoiceStatus::Overdue => &[InvoiceStatus::Paid, InvoiceStatus::Voided],
            InvoiceStatus::Paid | InvoiceStatus::Voided => &[],
        }
    }

    pub fn can_transition_to(&self, next: &InvoiceStatus) -> bool {
        self.allowed_transitions().contains(next)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use time::Month;

    fn date() -> Date {
        Date::from_calendar_date(2025, Month::January, 15).unwrap()
    }

    fn make_line(quantity: i64, unit_price: i64, tax_rate: i64) -> InvoiceLine {
        InvoiceLine {
            id: "line-id".to_string(),
            invoice_id: "inv-id".to_string(),
            description: "Widget".to_string(),
            account_id: None,
            quantity,
            unit_price,
            tax_rate,
            sort_order: 0,
        }
    }

    // ── InvoiceType ───────────────────────────────────────────────────────────

    #[test]
    fn invoice_type_display() {
        assert_eq!(InvoiceType::Invoice.to_string(), "invoice");
        assert_eq!(InvoiceType::Bill.to_string(), "bill");
    }

    // ── InvoiceStatus ─────────────────────────────────────────────────────────

    #[test]
    fn invoice_status_display() {
        assert_eq!(InvoiceStatus::Draft.to_string(), "draft");
        assert_eq!(InvoiceStatus::Sent.to_string(), "sent");
        assert_eq!(InvoiceStatus::Partial.to_string(), "partial");
        assert_eq!(InvoiceStatus::Paid.to_string(), "paid");
        assert_eq!(InvoiceStatus::Overdue.to_string(), "overdue");
        assert_eq!(InvoiceStatus::Voided.to_string(), "voided");
    }

    #[test]
    fn invoice_status_from_str_valid() {
        assert_eq!(
            InvoiceStatus::from_str("draft").unwrap(),
            InvoiceStatus::Draft
        );
        assert_eq!(
            InvoiceStatus::from_str("sent").unwrap(),
            InvoiceStatus::Sent
        );
        assert_eq!(
            InvoiceStatus::from_str("partial").unwrap(),
            InvoiceStatus::Partial
        );
        assert_eq!(
            InvoiceStatus::from_str("paid").unwrap(),
            InvoiceStatus::Paid
        );
        assert_eq!(
            InvoiceStatus::from_str("overdue").unwrap(),
            InvoiceStatus::Overdue
        );
        assert_eq!(
            InvoiceStatus::from_str("voided").unwrap(),
            InvoiceStatus::Voided
        );
    }

    #[test]
    fn invoice_status_from_str_invalid() {
        assert!(InvoiceStatus::from_str("").is_err());
        assert!(InvoiceStatus::from_str("Paid").is_err());
        assert!(InvoiceStatus::from_str("cancelled").is_err());
    }

    #[test]
    fn invoice_status_roundtrip() {
        for s in [
            InvoiceStatus::Draft,
            InvoiceStatus::Sent,
            InvoiceStatus::Partial,
            InvoiceStatus::Paid,
            InvoiceStatus::Overdue,
            InvoiceStatus::Voided,
        ] {
            let parsed = InvoiceStatus::from_str(&s.to_string()).unwrap();
            assert_eq!(parsed, s);
        }
    }

    // ── InvoiceLine calculations ───────────────────────────────────────────────

    #[test]
    fn line_total_whole_units_no_tax() {
        // 2 units × $10.00 = $20.00
        let line = make_line(200, 1_000, 0);
        assert_eq!(line.line_total(), 2_000);
        assert_eq!(line.tax_amount(), 0);
    }

    #[test]
    fn line_total_fractional_quantity() {
        // 1.5 units × $100.00 = $150.00
        let line = make_line(150, 10_000, 0);
        assert_eq!(line.line_total(), 15_000);
    }

    #[test]
    fn tax_amount_ten_percent() {
        // 1 unit × $100.00 at 10% = $10.00 tax
        let line = make_line(100, 10_000, 1_000);
        assert_eq!(line.line_total(), 10_000);
        assert_eq!(line.tax_amount(), 1_000);
    }

    #[test]
    fn tax_amount_zero_rate() {
        let line = make_line(100, 5_000, 0);
        assert_eq!(line.tax_amount(), 0);
    }

    #[test]
    fn tax_amount_twenty_percent() {
        // 1 unit × $50.00 at 20% = $10.00 tax
        let line = make_line(100, 5_000, 2_000);
        assert_eq!(line.tax_amount(), 1_000);
    }

    // ── CreateInvoice::validate ───────────────────────────────────────────────

    #[test]
    fn validate_ok_with_valid_lines() {
        let input = CreateInvoice {
            contact_id: "c1".to_string(),
            invoice_type: InvoiceType::Invoice,
            date: date(),
            due_date: date(),
            currency: None,
            notes: None,
            lines: vec![CreateInvoiceLine {
                description: "Widget".to_string(),
                account_id: None,
                quantity: 100,
                unit_price: 5_000,
                tax_rate: None,
            }],
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn validate_ok_with_empty_lines() {
        let input = CreateInvoice {
            contact_id: "c1".to_string(),
            invoice_type: InvoiceType::Invoice,
            date: date(),
            due_date: date(),
            currency: None,
            notes: None,
            lines: vec![],
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn validate_rejects_zero_quantity() {
        let input = CreateInvoice {
            contact_id: "c1".to_string(),
            invoice_type: InvoiceType::Invoice,
            date: date(),
            due_date: date(),
            currency: None,
            notes: None,
            lines: vec![CreateInvoiceLine {
                description: "Bad".to_string(),
                account_id: None,
                quantity: 0,
                unit_price: 1_000,
                tax_rate: None,
            }],
        };
        assert!(matches!(input.validate(), Err(CoreError::ZeroQuantity)));
    }

    #[test]
    fn validate_rejects_negative_quantity() {
        let input = CreateInvoice {
            contact_id: "c1".to_string(),
            invoice_type: InvoiceType::Invoice,
            date: date(),
            due_date: date(),
            currency: None,
            notes: None,
            lines: vec![CreateInvoiceLine {
                description: "Bad".to_string(),
                account_id: None,
                quantity: -1,
                unit_price: 1_000,
                tax_rate: None,
            }],
        };
        assert!(matches!(input.validate(), Err(CoreError::ZeroQuantity)));
    }

    #[test]
    fn validate_rejects_negative_unit_price() {
        let input = CreateInvoice {
            contact_id: "c1".to_string(),
            invoice_type: InvoiceType::Invoice,
            date: date(),
            due_date: date(),
            currency: None,
            notes: None,
            lines: vec![CreateInvoiceLine {
                description: "Bad".to_string(),
                account_id: None,
                quantity: 100,
                unit_price: -1,
                tax_rate: None,
            }],
        };
        assert!(matches!(
            input.validate(),
            Err(CoreError::NegativeAmount(-1))
        ));
    }

    #[test]
    fn validate_allows_zero_unit_price() {
        let input = CreateInvoice {
            contact_id: "c1".to_string(),
            invoice_type: InvoiceType::Invoice,
            date: date(),
            due_date: date(),
            currency: None,
            notes: None,
            lines: vec![CreateInvoiceLine {
                description: "Free item".to_string(),
                account_id: None,
                quantity: 100,
                unit_price: 0,
                tax_rate: None,
            }],
        };
        assert!(input.validate().is_ok());
    }
}
