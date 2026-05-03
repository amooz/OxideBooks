use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub rows_ok: usize,
    pub rows_failed: usize,
    pub errors: Vec<ImportError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportError {
    pub row: usize,
    pub message: String,
}

/// A single row from a contact CSV import.
/// Required columns: name. Optional: email, phone, contact_type, currency.
#[derive(Debug, Clone, Deserialize)]
pub struct ContactCsvRow {
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub contact_type: Option<String>,
    pub currency: Option<String>,
}
