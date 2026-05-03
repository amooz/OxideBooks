use axum::{
    extract::{Extension, State},
    Json,
};
use oxidebooks_core::models::{ExpenseStatus, InvoiceStatus, UpdateInvoice};
use oxidebooks_db::repos::{ExpenseRepo, InvoiceRepo};
use serde::{Deserialize, Serialize};

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Deserialize)]
pub struct BulkIds {
    pub ids: Vec<String>,
}

#[derive(Serialize)]
pub struct BulkResult {
    pub succeeded: Vec<String>,
    pub failed: Vec<BulkFailure>,
}

#[derive(Serialize)]
pub struct BulkFailure {
    pub id: String,
    pub reason: String,
}

pub async fn bulk_void_invoices(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<BulkIds>,
) -> ApiResult<Json<BulkResult>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let mut succeeded = Vec::new();
    let mut failed = Vec::new();
    for id in body.ids {
        match InvoiceRepo::update(
            &state.db,
            &claims.org,
            &id,
            UpdateInvoice {
                status: Some(InvoiceStatus::Voided),
                due_date: None,
                expiry_date: None,
                notes: None,
            },
        )
        .await
        {
            Ok(_) => succeeded.push(id),
            Err(e) => failed.push(BulkFailure {
                id,
                reason: e.to_string(),
            }),
        }
    }
    Ok(Json(BulkResult { succeeded, failed }))
}

pub async fn bulk_send_invoices(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<BulkIds>,
) -> ApiResult<Json<BulkResult>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let mut succeeded = Vec::new();
    let mut failed = Vec::new();
    for id in body.ids {
        match InvoiceRepo::update(
            &state.db,
            &claims.org,
            &id,
            UpdateInvoice {
                status: Some(InvoiceStatus::Sent),
                due_date: None,
                expiry_date: None,
                notes: None,
            },
        )
        .await
        {
            Ok(_) => succeeded.push(id),
            Err(e) => failed.push(BulkFailure {
                id,
                reason: e.to_string(),
            }),
        }
    }
    Ok(Json(BulkResult { succeeded, failed }))
}

pub async fn bulk_approve_expenses(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<BulkIds>,
) -> ApiResult<Json<BulkResult>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let mut succeeded = Vec::new();
    let mut failed = Vec::new();
    for id in body.ids {
        match ExpenseRepo::transition(&state.db, &claims.org, &id, ExpenseStatus::Approved).await {
            Ok(_) => succeeded.push(id),
            Err(e) => failed.push(BulkFailure {
                id,
                reason: e.to_string(),
            }),
        }
    }
    Ok(Json(BulkResult { succeeded, failed }))
}

pub async fn bulk_reject_expenses(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<BulkIds>,
) -> ApiResult<Json<BulkResult>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let mut succeeded = Vec::new();
    let mut failed = Vec::new();
    for id in body.ids {
        match ExpenseRepo::transition(&state.db, &claims.org, &id, ExpenseStatus::Rejected).await {
            Ok(_) => succeeded.push(id),
            Err(e) => failed.push(BulkFailure {
                id,
                reason: e.to_string(),
            }),
        }
    }
    Ok(Json(BulkResult { succeeded, failed }))
}
