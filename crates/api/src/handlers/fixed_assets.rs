use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::Response,
    Json,
};
use oxidebooks_core::models::{CreateFixedAsset, UpdateFixedAsset};
use oxidebooks_db::repos::FixedAssetRepo;
use serde::Deserialize;
use time::format_description::well_known::Iso8601;
use time::Date;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
    xlsx::{build_xlsx, XLSX_CONTENT_TYPE},
};

pub async fn list_fixed_assets(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("fixed_assets:read") {
        return Err(ApiError::Forbidden);
    }
    let assets = FixedAssetRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": assets })))
}

pub async fn get_fixed_asset(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("fixed_assets:read") {
        return Err(ApiError::Forbidden);
    }
    let asset = FixedAssetRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(asset)))
}

pub async fn create_fixed_asset(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateFixedAsset>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let asset = FixedAssetRepo::create(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(asset))))
}

pub async fn update_fixed_asset(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateFixedAsset>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let asset = FixedAssetRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!(asset)))
}

#[derive(Debug, Deserialize)]
pub struct DepreciateBody {
    pub period_date: String,
}

pub async fn depreciate_asset(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<DepreciateBody>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let period = Date::parse(&body.period_date, &Iso8601::DEFAULT)
        .map_err(|_| ApiError::BadRequest("invalid period_date, expected YYYY-MM-DD".into()))?;
    let asset = FixedAssetRepo::depreciate(&state.db, &claims.org, &id, period).await?;
    Ok(Json(serde_json::json!(asset)))
}

#[derive(Debug, Deserialize)]
pub struct DisposeBody {
    pub disposal_date: String,
}

pub async fn dispose_asset(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<DisposeBody>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let disposal = Date::parse(&body.disposal_date, &Iso8601::DEFAULT)
        .map_err(|_| ApiError::BadRequest("invalid disposal_date, expected YYYY-MM-DD".into()))?;
    let asset = FixedAssetRepo::dispose(&state.db, &claims.org, &id, disposal).await?;
    Ok(Json(serde_json::json!(asset)))
}

pub async fn asset_register(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("fixed_assets:read") {
        return Err(ApiError::Forbidden);
    }
    let rows = FixedAssetRepo::asset_register(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": rows })))
}

pub async fn get_depreciation_schedule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("fixed_assets:read") {
        return Err(ApiError::Forbidden);
    }
    let schedule = FixedAssetRepo::schedule(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": schedule })))
}

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    pub format: Option<String>,
}

pub async fn export_depreciation_schedule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Query(q): Query<ExportQuery>,
) -> ApiResult<Response> {
    if !claims.has("fixed_assets:read") {
        return Err(ApiError::Forbidden);
    }
    let asset = FixedAssetRepo::get_by_id(&state.db, &claims.org, &id).await?;
    let schedule = FixedAssetRepo::schedule(&state.db, &claims.org, &id).await?;

    let headers = &[
        "Period",
        "Date",
        "Depreciation",
        "Accumulated Depreciation",
        "Book Value",
        "Posted",
    ];

    let rows: Vec<Vec<String>> = schedule
        .iter()
        .map(|l| {
            vec![
                l.period.to_string(),
                l.period_date.to_string(),
                format!("{:.2}", l.amount as f64 / 100.0),
                format!("{:.2}", l.accumulated_depreciation as f64 / 100.0),
                format!("{:.2}", l.book_value as f64 / 100.0),
                if l.is_posted { "Yes" } else { "No" }.to_string(),
            ]
        })
        .collect();

    let filename = format!("depreciation-schedule-{}.xlsx", asset.asset_number);

    match q.format.as_deref() {
        Some("xlsx") => {
            let bytes = build_xlsx(
                &format!("Schedule {}", asset.asset_number),
                headers,
                &rows,
                &[2, 3, 4],
            );
            Ok(Response::builder()
                .header("Content-Type", XLSX_CONTENT_TYPE)
                .header(
                    "Content-Disposition",
                    format!("attachment; filename=\"{filename}\""),
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
            let csv_filename = filename.replace(".xlsx", ".csv");
            Ok(Response::builder()
                .header("Content-Type", "text/csv")
                .header(
                    "Content-Disposition",
                    format!("attachment; filename=\"{csv_filename}\""),
                )
                .body(axum::body::Body::from(csv))
                .unwrap())
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct BulkDepreciateBody {
    pub period_date: String,
}

pub async fn bulk_depreciate(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<BulkDepreciateBody>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let period = Date::parse(&body.period_date, &Iso8601::DEFAULT)
        .map_err(|_| ApiError::BadRequest("invalid period_date, expected YYYY-MM-DD".into()))?;
    let result = FixedAssetRepo::bulk_depreciate(&state.db, &claims.org, period).await?;
    Ok(Json(serde_json::json!({ "data": result })))
}
