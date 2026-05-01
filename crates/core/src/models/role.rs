use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    /// `None` for system roles visible to every org; `Some(id)` for org-custom roles.
    pub org_id: Option<String>,
    pub name: String,
    pub is_system: bool,
    /// Resolved permission names for this role.
    pub permissions: Vec<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateRole {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssignPermission {
    pub permission: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::OffsetDateTime;

    fn make_role(permissions: Vec<&str>) -> Role {
        Role {
            id: "00000000-0000-0000-0000-000000000001".to_string(),
            org_id: None,
            name: "test".to_string(),
            is_system: false,
            permissions: permissions.into_iter().map(String::from).collect(),
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        }
    }

    #[test]
    fn role_serializes_with_permissions() {
        let role = make_role(vec!["accounts:read", "accounts:write"]);
        let json = serde_json::to_value(&role).unwrap();
        let perms = json["permissions"].as_array().unwrap();
        assert_eq!(perms.len(), 2);
        assert_eq!(perms[0], "accounts:read");
        assert_eq!(perms[1], "accounts:write");
    }

    #[test]
    fn role_serializes_system_role_with_null_org_id() {
        let role = make_role(vec![]);
        let json = serde_json::to_value(&role).unwrap();
        assert!(json["org_id"].is_null());
    }

    #[test]
    fn role_serializes_custom_role_with_org_id() {
        let mut role = make_role(vec![]);
        role.org_id = Some("org-123".to_string());
        let json = serde_json::to_value(&role).unwrap();
        assert_eq!(json["org_id"], "org-123");
    }

    #[test]
    fn create_role_deserializes() {
        let json = r#"{"name": "billing-manager"}"#;
        let cr: CreateRole = serde_json::from_str(json).unwrap();
        assert_eq!(cr.name, "billing-manager");
    }

    #[test]
    fn assign_permission_deserializes() {
        let json = r#"{"permission": "invoices:write"}"#;
        let ap: AssignPermission = serde_json::from_str(json).unwrap();
        assert_eq!(ap.permission, "invoices:write");
    }

    #[test]
    fn permission_serializes() {
        let p = Permission {
            id: "abc".to_string(),
            name: "accounts:read".to_string(),
            description: Some("View chart of accounts".to_string()),
        };
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["name"], "accounts:read");
        assert_eq!(json["description"], "View chart of accounts");
    }

    #[test]
    fn permission_with_no_description_serializes_null() {
        let p = Permission {
            id: "abc".to_string(),
            name: "roles:write".to_string(),
            description: None,
        };
        let json = serde_json::to_value(&p).unwrap();
        assert!(json["description"].is_null());
    }
}
