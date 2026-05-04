use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{
    CreateBillLine, CreatePoLine, CreatePurchaseOrder, CreateVendorBill, ReceivePoLine,
    UpdatePurchaseOrder,
};
use oxidebooks_db::repos::{BillRepo, PurchaseOrderRepo};
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct PoQuery {
    pub status: Option<String>,
}

pub async fn list_purchase_orders(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<PoQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("purchase_orders:read") {
        return Err(ApiError::Forbidden);
    }
    let pos = PurchaseOrderRepo::list(&state.db, &claims.org, q.status.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": pos })))
}

pub async fn get_purchase_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("purchase_orders:read") {
        return Err(ApiError::Forbidden);
    }
    let po = PurchaseOrderRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(po)))
}

pub async fn create_purchase_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreatePurchaseOrder>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let po = PurchaseOrderRepo::create(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(po))))
}

pub async fn update_purchase_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdatePurchaseOrder>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let po = PurchaseOrderRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!(po)))
}

pub async fn delete_purchase_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    PurchaseOrderRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/purchase-orders/:id/approve
pub async fn approve_purchase_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let po = PurchaseOrderRepo::approve(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": po })))
}

pub async fn receive_purchase_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<Vec<ReceivePoLine>>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let po = PurchaseOrderRepo::receive(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!(po)))
}

pub async fn add_po_line(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<CreatePoLine>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let po = PurchaseOrderRepo::add_line(&state.db, &claims.org, &id, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(po))))
}

/// POST /api/v1/purchase-orders/:id/create-bill
///
/// Converts a received purchase order into a draft vendor bill. Requires the PO
/// to have at least one received line (status `received` or `partially_received`).
pub async fn po_create_bill(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let po = PurchaseOrderRepo::get_by_id(&state.db, &claims.org, &id).await?;
    let status = po.status.to_string();
    if status != "received" && status != "partially_received" {
        return Err(ApiError::BadRequest(
            "PO must be received or partially received before creating a bill".into(),
        ));
    }

    let today = time::OffsetDateTime::now_utc().date();
    let lines: Vec<CreateBillLine> = po
        .lines
        .iter()
        .filter(|l| l.quantity_received > 0)
        .map(|l| CreateBillLine {
            account_id: None,
            description: Some(l.description.clone()),
            quantity: l.quantity_received as i32,
            unit_price: l.unit_price,
            tax_rate: l.tax_rate,
        })
        .collect();

    if lines.is_empty() {
        return Err(ApiError::BadRequest("no received lines to bill".into()));
    }

    let input = CreateVendorBill {
        contact_id: Some(po.contact_id.clone()),
        bill_date: today,
        due_date: None,
        reference: Some(po.po_number.clone()),
        description: format!("Bill for PO {}", po.po_number),
        currency_code: "USD".into(),
        exchange_rate: rust_decimal::Decimal::ONE,
        lines,
        purchase_order_id: Some(po.id.clone()),
    };

    let bill = BillRepo::create(&state.db, &claims.org, input).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": bill })),
    ))
}
