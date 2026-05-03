use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{ConvertSoToInvoice, CreateSalesOrder, UpdateSalesOrder};
use oxidebooks_db::repos::SalesOrderRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct SoQuery {
    pub status: Option<String>,
    pub contact_id: Option<String>,
}

pub async fn list_sales_orders(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<SoQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("sales_orders:read") {
        return Err(ApiError::Forbidden);
    }
    let orders = SalesOrderRepo::list(
        &state.db,
        &claims.org,
        q.status.as_deref(),
        q.contact_id.as_deref(),
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": orders })))
}

pub async fn get_sales_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("sales_orders:read") {
        return Err(ApiError::Forbidden);
    }
    let so = SalesOrderRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(so)))
}

pub async fn create_sales_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateSalesOrder>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let so = SalesOrderRepo::create(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(so))))
}

pub async fn update_sales_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateSalesOrder>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let so = SalesOrderRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!(so)))
}

pub async fn confirm_sales_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let so = SalesOrderRepo::confirm(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(so)))
}

pub async fn cancel_sales_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let so = SalesOrderRepo::cancel(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(so)))
}

pub async fn convert_so_to_invoice(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<ConvertSoToInvoice>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let invoice = SalesOrderRepo::convert_to_invoice(&state.db, &claims.org, &id, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(invoice))))
}
