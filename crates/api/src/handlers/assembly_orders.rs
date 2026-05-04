use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::CreateAssemblyOrder;
use oxidebooks_db::repos::AssemblyOrderRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/assembly-orders
pub async fn list_assembly_orders(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let orders = AssemblyOrderRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": orders })))
}

/// GET /api/v1/assembly-orders/:id
pub async fn get_assembly_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let order = AssemblyOrderRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": order })))
}

/// GET /api/v1/assembly-orders/:id/lines
pub async fn list_assembly_order_lines(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let lines = AssemblyOrderRepo::list_lines(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": lines })))
}

/// POST /api/v1/assembly-orders
pub async fn create_assembly_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateAssemblyOrder>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let order = AssemblyOrderRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": order })),
    ))
}

/// POST /api/v1/assembly-orders/:id/build
pub async fn build_assembly_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let order = AssemblyOrderRepo::build(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": order })))
}

/// POST /api/v1/assembly-orders/:id/cancel
pub async fn cancel_assembly_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let order = AssemblyOrderRepo::cancel(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": order })))
}
