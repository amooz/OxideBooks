use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

pub const DEFAULT_LIMIT: i64 = 50;
pub const MAX_LIMIT: i64 = 200;

#[derive(Debug, Clone, Deserialize)]
pub struct PageParams {
    #[serde(default = "default_limit")]
    pub limit: i64,
    pub after: Option<String>,
}

fn default_limit() -> i64 {
    DEFAULT_LIMIT
}

impl PageParams {
    pub fn limit_clamped(&self) -> i64 {
        self.limit.clamp(1, MAX_LIMIT)
    }

    pub fn decode_cursor(&self) -> Option<PageCursor> {
        self.after.as_deref().and_then(decode_cursor)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageCursor {
    pub created_at: String, // RFC 3339 / ISO 8601
    pub id: String,
}

pub fn encode_cursor(created_at: OffsetDateTime, id: &str) -> String {
    let fmt = time::format_description::well_known::Rfc3339;
    let ts = created_at.format(&fmt).unwrap_or_default();
    let raw = serde_json::json!({ "created_at": ts, "id": id }).to_string();
    URL_SAFE_NO_PAD.encode(raw.as_bytes())
}

pub fn decode_cursor(encoded: &str) -> Option<PageCursor> {
    let bytes = URL_SAFE_NO_PAD.decode(encoded).ok()?;
    serde_json::from_slice(&bytes).ok()
}

#[derive(Debug, Serialize)]
pub struct Pagination {
    pub has_next: bool,
    pub next_cursor: Option<String>,
}
