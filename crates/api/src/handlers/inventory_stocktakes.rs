use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateInventoryStocktake, UpdateStocktakeLine};
use oxidebooks_db::repos::InventoryStocktakeRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Deserialize)]
pub struct StocktakeQuery {
    pub status: Option<String>,
}

pub async fn list_stocktakes(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<StocktakeQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let items = InventoryStocktakeRepo::list(&state.db, &claims.org, q.status.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": items })))
}

pub async fn get_stocktake(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let item = InventoryStocktakeRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": item })))
}

pub async fn create_stocktake(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateInventoryStocktake>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let item = InventoryStocktakeRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": item })),
    ))
}

pub async fn update_stocktake_line(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((stocktake_id, line_id)): Path<(String, String)>,
    Json(body): Json<UpdateStocktakeLine>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let item =
        InventoryStocktakeRepo::update_line(&state.db, &claims.org, &stocktake_id, &line_id, body)
            .await?;
    Ok(Json(serde_json::json!({ "data": item })))
}

pub async fn submit_stocktake(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let item = InventoryStocktakeRepo::submit(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": item })))
}

pub async fn post_stocktake(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let item = InventoryStocktakeRepo::post(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": item })))
}
