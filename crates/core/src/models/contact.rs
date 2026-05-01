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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contact_type_display() {
        assert_eq!(ContactType::Customer.to_string(), "customer");
        assert_eq!(ContactType::Vendor.to_string(), "vendor");
        assert_eq!(ContactType::Both.to_string(), "both");
    }

    #[test]
    fn contact_type_serde_roundtrip() {
        for ct in [ContactType::Customer, ContactType::Vendor, ContactType::Both] {
            let json = serde_json::to_string(&ct).unwrap();
            let parsed: ContactType = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, ct);
        }
    }

    #[test]
    fn create_contact_optional_type_defaults_handled_by_repo() {
        // ContactType is optional in CreateContact; the repo defaults to Both.
        let c = CreateContact {
            name: "Acme Corp".to_string(),
            contact_type: None,
            email: Some("acme@example.com".to_string()),
            phone: None,
            address: None,
            tax_number: None,
            currency: None,
        };
        assert!(c.contact_type.is_none());
        assert_eq!(c.email.as_deref(), Some("acme@example.com"));
    }

    #[test]
    fn update_contact_all_none_is_noop() {
        // An all-None update is valid; repos use COALESCE to leave fields unchanged.
        let u = UpdateContact::default();
        assert!(u.name.is_none());
        assert!(u.is_active.is_none());
    }
}
