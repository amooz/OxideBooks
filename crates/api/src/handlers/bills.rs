use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{
    CreateBillPayment, CreateSpendMoney, CreateVendorBill, UpdateVendorBill,
};
use oxidebooks_db::repos::BillRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/bills
pub async fn list_bills(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let bills = BillRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": bills })))
}

/// GET /api/v1/bills/:id
pub async fn get_bill(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let bill = BillRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": bill })))
}

/// POST /api/v1/bills
pub async fn create_bill(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateVendorBill>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let bill = BillRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": bill })),
    ))
}

/// PATCH /api/v1/bills/:id
pub async fn update_bill(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateVendorBill>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let bill = BillRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": bill })))
}

/// POST /api/v1/bills/:id/approve
pub async fn approve_bill(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let bill = BillRepo::approve(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": bill })))
}

/// POST /api/v1/bills/:id/void
pub async fn void_bill(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let bill = BillRepo::void(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": bill })))
}

/// POST /api/v1/bills/:id/payments
pub async fn create_bill_payment(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<CreateBillPayment>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let payment = BillRepo::record_payment(&state.db, &claims.org, &id, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": payment })),
    ))
}

/// GET /api/v1/bills/:id/payments
pub async fn list_bill_payments(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let payments = BillRepo::list_payments(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": payments })))
}

/// POST /api/v1/bills/spend-money
/// Creates a vendor bill + full payment atomically (no open AP).
pub async fn create_spend_money(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateSpendMoney>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }

    let payment_date = body.date;
    let payment_method = body.payment_method.clone();
    let reference = body.reference.clone();

    let bill_input = CreateVendorBill {
        contact_id: body.contact_id,
        bill_date: payment_date,
        due_date: Some(payment_date),
        reference: reference.clone(),
        description: body.description,
        currency_code: body.currency_code,
        exchange_rate: body.exchange_rate,
        lines: body.lines,
    };

    let bill = BillRepo::create(&state.db, &claims.org, bill_input).await?;

    let payment = BillRepo::record_payment(
        &state.db,
        &claims.org,
        &bill.id,
        CreateBillPayment {
            payment_date,
            amount: bill.total,
            method: payment_method,
            reference,
        },
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": { "bill": bill, "payment": payment } })),
    ))
}
