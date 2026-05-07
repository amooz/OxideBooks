use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EInvoiceTransmission {
    pub id: String,
    pub organization_id: String,
    pub invoice_id: String,
    pub format: String,
    pub status: String,
    pub external_id: Option<String>,
    pub transmission_xml: Option<String>,
    pub error_message: Option<String>,
    pub sent_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SendEInvoice {
    /// "ubl" or "peppol" (defaults to "ubl")
    pub format: Option<String>,
    /// Optional recipient endpoint override
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InboundEInvoice {
    /// Raw UBL 2.1 XML body
    pub xml: String,
}
