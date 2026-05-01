use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactType {
    Customer,
    Vendor,
    Both,
}

impl std::fmt::Display for ContactType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ContactType::Customer => "customer",
            ContactType::Vendor => "vendor",
            ContactType::Both => "both",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub contact_type: ContactType,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub tax_number: Option<String>,
    pub currency: Option<String>,
    pub is_active: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateContact {
    pub name: String,
    pub contact_type: Option<ContactType>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub tax_number: Option<String>,
    pub currency: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct UpdateContact {
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub tax_number: Option<String>,
    pub currency: Option<String>,
    pub is_active: Option<bool>,
}
