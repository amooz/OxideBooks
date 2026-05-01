pub mod account;
pub mod contact;
pub mod invoice;
pub mod organization;
pub mod reports;
pub mod transaction;

pub use account::{Account, AccountType, CreateAccount, UpdateAccount};
pub use contact::{Contact, ContactType, CreateContact, UpdateContact};
pub use invoice::{
    CreateInvoice, CreateInvoiceLine, Invoice, InvoiceLine, InvoiceStatus, InvoiceType,
};
pub use organization::{CreateOrganization, Organization};
pub use reports::{AccountBalance, TrialBalance};
pub use transaction::{
    CreateJournalEntry, CreateJournalLine, JournalEntry, JournalEntryStatus, JournalLine,
};

/// Serde helpers for `time::Date` as `"YYYY-MM-DD"`.
pub mod date_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::{format_description, Date};

    pub fn serialize<S: Serializer>(date: &Date, s: S) -> Result<S::Ok, S::Error> {
        let fmt = format_description::parse("[year]-[month]-[day]")
            .expect("static format is valid");
        s.serialize_str(&date.format(&fmt).map_err(serde::ser::Error::custom)?)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Date, D::Error> {
        let raw = String::deserialize(d)?;
        let fmt = format_description::parse("[year]-[month]-[day]")
            .expect("static format is valid");
        Date::parse(&raw, &fmt).map_err(serde::de::Error::custom)
    }
}
