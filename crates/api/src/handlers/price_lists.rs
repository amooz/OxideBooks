use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreatePriceList, UpsertPriceListItem};
use oxidebooks_db::repos::PriceListRepo;
use serde::Deserialize;
use time::Date;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

pub async fn list_price_lists(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    let lists = PriceListRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": lists })))
}

pub async fn create_price_list(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreatePriceList>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let list = PriceListRepo::create(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(list))))
}

pub async fn delete_price_list(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    PriceListRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_price_list_items(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let items = PriceListRepo::list_items(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": items })))
}

pub async fn upsert_price_list_item(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpsertPriceListItem>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let item = PriceListRepo::upsert_item(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!(item)))
}

#[derive(Deserialize)]
pub struct SpendAnalysisParams {
    pub from: String,
    pub to: String,
}

pub async fn spend_analysis(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<SpendAnalysisParams>,
) -> ApiResult<Json<serde_json::Value>> {
    let fmt = time::format_description::parse("[year]-[month]-[day]")
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;
    let from = Date::parse(&params.from, &fmt)
        .map_err(|_| ApiError::BadRequest("invalid 'from' date".into()))?;
    let to = Date::parse(&params.to, &fmt)
        .map_err(|_| ApiError::BadRequest("invalid 'to' date".into()))?;
    let report = PriceListRepo::spend_analysis(&state.db, &claims.org, from, to).await?;
    Ok(Json(serde_json::json!(report)))
}
