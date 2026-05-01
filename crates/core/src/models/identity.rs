use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Oidc,
    Saml,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::Oidc => f.write_str("oidc"),
            ProviderType::Saml => f.write_str("saml"),
        }
    }
}

impl std::str::FromStr for ProviderType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "oidc" => Ok(ProviderType::Oidc),
            "saml" => Ok(ProviderType::Saml),
            _ => Err(format!("unknown provider type: {s}")),
        }
    }
}

/// A configured external identity provider (OIDC or SAML).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityProvider {
    pub id: String,
    pub org_id: String,
    pub name: String,
    pub provider_type: ProviderType,
    pub is_enabled: bool,
    pub email_domains: Vec<String>,

    // OIDC fields (present when provider_type == oidc)
    pub oidc_client_id: Option<String>,
    #[serde(skip_serializing)] // never return the secret
    pub oidc_client_secret: Option<String>,
    pub oidc_issuer_url: Option<String>,
    pub oidc_scopes: String,

    // SAML fields (present when provider_type == saml)
    pub saml_idp_metadata_url: Option<String>,
    pub saml_idp_entity_id: Option<String>,
    pub saml_idp_sso_url: Option<String>,
    #[serde(skip_serializing)] // PEM cert — stored but never returned in API responses
    pub saml_idp_certificate: Option<String>,
    pub saml_sp_entity_id: Option<String>,

    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateOidcProvider {
    pub name: String,
    pub client_id: String,
    pub client_secret: String,
    pub issuer_url: String,
    pub scopes: Option<String>,
    pub email_domains: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSamlProvider {
    pub name: String,
    pub idp_metadata_url: Option<String>,
    pub idp_entity_id: Option<String>,
    pub idp_sso_url: Option<String>,
    pub idp_certificate: Option<String>,
    pub sp_entity_id: Option<String>,
    pub email_domains: Option<Vec<String>>,
}

/// Public info about a provider (safe to expose to the login page).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSummary {
    pub id: String,
    pub name: String,
    pub provider_type: ProviderType,
}

/// Represents a SCIM bearer token (the raw token is only returned at creation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimToken {
    pub id: String,
    pub org_id: String,
    pub name: String,
    pub is_active: bool,
    pub last_used_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Returned once at creation; the raw token is never stored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatedScimToken {
    #[serde(flatten)]
    pub token: ScimToken,
    /// Raw bearer token — show once, never again.
    pub raw_token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateScimToken {
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_type_display() {
        assert_eq!(ProviderType::Oidc.to_string(), "oidc");
        assert_eq!(ProviderType::Saml.to_string(), "saml");
    }

    #[test]
    fn provider_type_from_str() {
        assert_eq!("oidc".parse::<ProviderType>().unwrap(), ProviderType::Oidc);
        assert_eq!("saml".parse::<ProviderType>().unwrap(), ProviderType::Saml);
        assert!("other".parse::<ProviderType>().is_err());
    }

    #[test]
    fn provider_type_serde_roundtrip() {
        let json = serde_json::to_string(&ProviderType::Oidc).unwrap();
        assert_eq!(json, r#""oidc""#);
        let parsed: ProviderType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ProviderType::Oidc);
    }

    #[test]
    fn create_oidc_provider_deserializes() {
        let json = r#"{
            "name": "Google",
            "client_id": "id123",
            "client_secret": "secret",
            "issuer_url": "https://accounts.google.com",
            "email_domains": ["example.com"]
        }"#;
        let p: CreateOidcProvider = serde_json::from_str(json).unwrap();
        assert_eq!(p.name, "Google");
        assert_eq!(p.email_domains.unwrap()[0], "example.com");
    }

    #[test]
    fn create_saml_provider_deserializes() {
        let json = r#"{
            "name": "Okta SAML",
            "idp_sso_url": "https://okta.example.com/sso/saml",
            "idp_certificate": "-----BEGIN CERTIFICATE-----\n..."
        }"#;
        let p: CreateSamlProvider = serde_json::from_str(json).unwrap();
        assert_eq!(p.name, "Okta SAML");
        assert!(p.idp_sso_url.is_some());
    }

    #[test]
    fn create_scim_token_deserializes() {
        let json = r#"{"name": "Okta SCIM"}"#;
        let t: CreateScimToken = serde_json::from_str(json).unwrap();
        assert_eq!(t.name, "Okta SCIM");
    }

    #[test]
    fn oidc_client_secret_is_not_serialized() {
        use time::OffsetDateTime;
        let idp = IdentityProvider {
            id: "1".into(),
            org_id: "org-1".into(),
            name: "Google".into(),
            provider_type: ProviderType::Oidc,
            is_enabled: true,
            email_domains: vec![],
            oidc_client_id: Some("client-id".into()),
            oidc_client_secret: Some("super-secret".into()),
            oidc_issuer_url: Some("https://accounts.google.com".into()),
            oidc_scopes: "openid email profile".into(),
            saml_idp_metadata_url: None,
            saml_idp_entity_id: None,
            saml_idp_sso_url: None,
            saml_idp_certificate: None,
            saml_sp_entity_id: None,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        };
        let json = serde_json::to_value(&idp).unwrap();
        assert!(json.get("oidc_client_secret").is_none());
        assert_eq!(json["oidc_client_id"], "client-id");
    }
}
