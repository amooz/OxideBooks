use axum::{
    extract::{Extension, Path, Query, State},
    response::Response,
    Json,
};
use oxidebooks_core::pagination::PageParams;
use oxidebooks_db::repos::AuditRepo;
use serde::Deserialize;
use time::OffsetDateTime;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
    xlsx::{build_xlsx, XLSX_CONTENT_TYPE},
};

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    #[serde(flatten)]
    pub page: PageParams,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub user_id: Option<String>,
    pub severity: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
}

fn parse_ts(s: &str) -> Result<OffsetDateTime, ApiError> {
    OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339)
        .map_err(|_| ApiError::BadRequest(format!("invalid timestamp: {s}")))
}

pub async fn list_audit_log(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<AuditQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let since = q.since.as_deref().map(parse_ts).transpose()?;
    let until = q.until.as_deref().map(parse_ts).transpose()?;

    let (events, next_cursor) = AuditRepo::list(
        &state.db,
        &claims.org,
        q.resource_type.as_deref(),
        q.resource_id.as_deref(),
        q.user_id.as_deref(),
        q.severity.as_deref(),
        since,
        until,
        &q.page,
    )
    .await?;
    Ok(Json(serde_json::json!({
        "data": events,
        "pagination": { "has_next": next_cursor.is_some(), "next_cursor": next_cursor }
    })))
}

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    pub format: Option<String>,
    pub resource_type: Option<String>,
    pub user_id: Option<String>,
    pub severity: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
}

pub async fn export_audit_log(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<ExportQuery>,
) -> ApiResult<Response> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let since = q.since.as_deref().map(parse_ts).transpose()?;
    let until = q.until.as_deref().map(parse_ts).transpose()?;

    let events = AuditRepo::list_for_export(
        &state.db,
        &claims.org,
        q.resource_type.as_deref(),
        q.user_id.as_deref(),
        q.severity.as_deref(),
        since,
        until,
    )
    .await?;

    let headers = &[
        "ID",
        "Timestamp",
        "User ID",
        "Action",
        "Resource Type",
        "Resource ID",
        "Severity",
        "IP Address",
        "User Agent",
    ];

    let rows: Vec<Vec<String>> = events
        .iter()
        .map(|e| {
            vec![
                e.id.clone(),
                e.created_at
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default(),
                e.user_id.clone().unwrap_or_default(),
                e.action.clone(),
                e.resource_type.clone(),
                e.resource_id.clone(),
                e.severity.clone(),
                e.ip_address.clone().unwrap_or_default(),
                e.user_agent.clone().unwrap_or_default(),
            ]
        })
        .collect();

    match q.format.as_deref() {
        Some("xlsx") => {
            let bytes = build_xlsx("Audit Log", headers, &rows, &[]);
            Ok(Response::builder()
                .header("Content-Type", XLSX_CONTENT_TYPE)
                .header(
                    "Content-Disposition",
                    "attachment; filename=\"audit-log.xlsx\"",
                )
                .body(axum::body::Body::from(bytes))
                .unwrap())
        }
        _ => {
            let mut csv = headers.join(",") + "\n";
            for row in &rows {
                let line = row
                    .iter()
                    .map(|v| format!("\"{}\"", v.replace('"', "\"\"")))
                    .collect::<Vec<_>>()
                    .join(",");
                csv.push_str(&line);
                csv.push('\n');
            }
            Ok(Response::builder()
                .header("Content-Type", "text/csv")
                .header(
                    "Content-Disposition",
                    "attachment; filename=\"audit-log.csv\"",
                )
                .body(axum::body::Body::from(csv))
                .unwrap())
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SummaryQuery {
    pub since: Option<String>,
    pub until: Option<String>,
}

pub async fn get_audit_for_resource(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((resource_type, resource_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let events =
        AuditRepo::get_for_resource(&state.db, &claims.org, &resource_type, &resource_id).await?;
    Ok(Json(serde_json::json!({ "data": events })))
}

#[derive(Debug, Deserialize)]
pub struct PurgeQuery {
    pub older_than_days: i64,
}

pub async fn purge_audit_log(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<PurgeQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    if q.older_than_days < 1 {
        return Err(ApiError::BadRequest("older_than_days must be >= 1".into()));
    }
    let deleted = AuditRepo::purge_old(&state.db, &claims.org, q.older_than_days).await?;
    Ok(Json(serde_json::json!({ "deleted": deleted })))
}

pub async fn compliance_summary(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<SummaryQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let since = q.since.as_deref().map(parse_ts).transpose()?;
    let until = q.until.as_deref().map(parse_ts).transpose()?;

    let summary = AuditRepo::compliance_summary(&state.db, &claims.org, since, until).await?;
    Ok(Json(serde_json::json!({ "data": summary })))
}
