use axum::{
    extract::{Extension, Query, State},
    Json,
};
use oxidebooks_db::repos::ReportRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct ConsolidatedQuery {
    /// Comma-separated list of org UUIDs to include.
    pub org_ids: String,
    pub from: Option<String>,
    pub to: Option<String>,
}

pub async fn consolidated_profit_loss(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<ConsolidatedQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }

    use time::{format_description, Date, Month, OffsetDateTime};
    let fmt = format_description::parse("[year]-[month]-[day]").expect("static fmt");
    let from = q
        .from
        .as_deref()
        .and_then(|s| Date::parse(s, &fmt).ok())
        .unwrap_or_else(|| {
            let now = OffsetDateTime::now_utc();
            Date::from_calendar_date(now.year(), Month::January, 1).unwrap()
        });
    let to =
        q.to.as_deref()
            .and_then(|s| Date::parse(s, &fmt).ok())
            .unwrap_or_else(|| OffsetDateTime::now_utc().date());

    let org_ids: Vec<&str> = q.org_ids.split(',').map(str::trim).collect();
    let report = ReportRepo::consolidated_profit_loss(&state.db, &org_ids, from, to).await?;
    Ok(Json(serde_json::json!(report)))
}
