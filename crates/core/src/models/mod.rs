pub mod account;
pub mod contact;
pub mod identity;
pub mod invoice;
pub mod organization;
pub mod payment;
pub mod reports;
pub mod role;
pub mod transaction;

pub use account::{Account, AccountType, CreateAccount, UpdateAccount};
pub use contact::{Contact, ContactType, CreateContact, UpdateContact};
pub use identity::{
    CreateOidcProvider, CreateSamlProvider, CreateScimToken, CreatedScimToken, IdentityProvider,
    ProviderSummary, ProviderType, ScimToken,
};
pub use invoice::{
    CreateInvoice, CreateInvoiceLine, Invoice, InvoiceFilters, InvoiceLine, InvoiceStatus,
    InvoiceType, UpdateInvoice,
};
pub use organization::{CreateOrganization, Organization, UpdateOrganization};
pub use payment::{CreatePayment, Payment, VALID_METHODS};
pub use reports::{
    AccountBalance, BalanceSheetReport, ProfitLossReport, ReportLine, ReportSection, TrialBalance,
};
pub use role::{AssignPermission, CreateRole, Permission, Role};
pub use transaction::{
    CreateJournalEntry, CreateJournalLine, JournalEntry, JournalEntryStatus, JournalLine,
};

/// Serde helpers for `Option<time::Date>` as `"YYYY-MM-DD"`.
pub mod opt_date_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use time::{format_description, Date};

    pub fn serialize<S: Serializer>(date: &Option<Date>, s: S) -> Result<S::Ok, S::Error> {
        match date {
            Some(d) => {
                let fmt = format_description::parse("[year]-[month]-[day]")
                    .expect("static format is valid");
                s.serialize_some(&d.format(&fmt).map_err(serde::ser::Error::custom)?)
            }
            None => Option::<String>::None.serialize(s),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Date>, D::Error> {
        let raw: Option<String> = Option::deserialize(d)?;
        match raw {
            None => Ok(None),
            Some(s) => {
                let fmt = format_description::parse("[year]-[month]-[day]")
                    .expect("static format is valid");
                Date::parse(&s, &fmt)
                    .map(Some)
                    .map_err(serde::de::Error::custom)
            }
        }
    }
}

/// Serde helpers for `time::Date` as `"YYYY-MM-DD"`.
pub mod date_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::{format_description, Date};

    pub fn serialize<S: Serializer>(date: &Date, s: S) -> Result<S::Ok, S::Error> {
        let fmt =
            format_description::parse("[year]-[month]-[day]").expect("static format is valid");
        s.serialize_str(&date.format(&fmt).map_err(serde::ser::Error::custom)?)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Date, D::Error> {
        let raw = String::deserialize(d)?;
        let fmt =
            format_description::parse("[year]-[month]-[day]").expect("static format is valid");
        Date::parse(&raw, &fmt).map_err(serde::de::Error::custom)
    }
}
