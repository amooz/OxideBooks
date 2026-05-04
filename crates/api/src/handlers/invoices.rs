use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{
    BillableExpenseRef, CreateCashSale, CreateInvoice, CreateInvoiceLine, CreatePayment,
    InvoiceFilters, InvoiceType, ProgressInvoiceInput, UpdateInvoice,
};
use oxidebooks_core::pagination::PageParams;
use oxidebooks_db::repos::{AuditRepo, ContactRepo, ExpenseRepo, InvoiceRepo, PaymentRepo};
use serde::Deserialize;
use tracing::info;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct InvoiceQuery {
    #[serde(flatten)]
    pub page: PageParams,
    pub status: Option<String>,
    #[serde(rename = "type")]
    pub invoice_type: Option<String>,
    pub contact_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

/// GET /api/v1/invoices
pub async fn list_invoices(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<InvoiceQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:read") {
        return Err(ApiError::Forbidden);
    }

    let parse_date = |s: &str| {
        let fmt = time::macros::format_description!("[year]-[month]-[day]");
        time::Date::parse(s, fmt)
            .map_err(|_| ApiError::BadRequest(format!("invalid date '{s}'; expected YYYY-MM-DD")))
    };

    let filters = InvoiceFilters {
        status: q.status,
        invoice_type: q.invoice_type,
        contact_id: q.contact_id,
        from: q.from.as_deref().map(parse_date).transpose()?,
        to: q.to.as_deref().map(parse_date).transpose()?,
    };

    let (invoices, next_cursor) =
        InvoiceRepo::list(&state.db, &claims.org, &q.page, &filters).await?;
    Ok(Json(serde_json::json!({
        "data": invoices,
        "pagination": { "has_next": next_cursor.is_some(), "next_cursor": next_cursor }
    })))
}

/// GET /api/v1/invoices/:id
pub async fn get_invoice(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:read") {
        return Err(ApiError::Forbidden);
    }
    let invoice = InvoiceRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": invoice })))
}

/// POST /api/v1/invoices
pub async fn create_invoice(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateInvoice>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("invoices:write") {
        return Err(ApiError::Forbidden);
    }

    // Credit limit enforcement
    if let Ok(contact) = ContactRepo::get_by_id(&state.db, &claims.org, &body.contact_id).await {
        if let Some(limit) = contact.credit_limit {
            if contact.credit_limit_behaviour == "block" {
                let ar_balance =
                    InvoiceRepo::contact_ar_balance(&state.db, &claims.org, &body.contact_id)
                        .await
                        .unwrap_or(0);
                if ar_balance >= limit {
                    return Err(ApiError::BadRequest(format!(
                        "contact has reached their credit limit of {limit} (current balance: {ar_balance})"
                    )));
                }
            }
        }
    }

    let invoice = InvoiceRepo::create(&state.db, &claims.org, body).await?;
    let _ = AuditRepo::record(
        &state.db,
        &claims.org,
        Some(&claims.sub),
        "create",
        "invoice",
        &invoice.id,
        None,
    )
    .await;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": invoice })),
    ))
}

/// PATCH /api/v1/invoices/:id
pub async fn update_invoice(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateInvoice>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:write") {
        return Err(ApiError::Forbidden);
    }
    let invoice = InvoiceRepo::update(&state.db, &claims.org, &id, body).await?;
    info!(invoice_id = %id, org_id = %claims.org, status = %invoice.status, "invoice updated");
    let _ = AuditRepo::record(
        &state.db,
        &claims.org,
        Some(&claims.sub),
        "update",
        "invoice",
        &id,
        None,
    )
    .await;
    Ok(Json(serde_json::json!({ "data": invoice })))
}

/// POST /api/v1/invoices/:id/convert
/// Convert an accepted quote into a new invoice.
pub async fn convert_quote(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("invoices:write") {
        return Err(ApiError::Forbidden);
    }
    let invoice = InvoiceRepo::convert_quote(&state.db, &claims.org, &id).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": invoice })),
    ))
}

#[derive(serde::Deserialize)]
pub struct ApplyCreditBody {
    pub invoice_id: String,
    pub amount: i64,
}

/// POST /api/v1/invoices/:id/apply-credit
/// Apply a credit note against a target invoice.
pub async fn apply_credit(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<ApplyCreditBody>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:write") {
        return Err(ApiError::Forbidden);
    }
    if body.amount <= 0 {
        return Err(ApiError::BadRequest("amount must be positive".into()));
    }
    let invoice =
        InvoiceRepo::apply_credit(&state.db, &claims.org, &id, &body.invoice_id, body.amount)
            .await?;
    Ok(Json(serde_json::json!({ "data": invoice })))
}

/// POST /api/v1/quotes/:id/accept
pub async fn accept_quote(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:write") {
        return Err(ApiError::Forbidden);
    }
    let invoice = InvoiceRepo::update_quote_status(&state.db, &claims.org, &id, "accepted").await?;
    Ok(Json(serde_json::json!({ "data": invoice })))
}

/// POST /api/v1/quotes/:id/decline
pub async fn decline_quote(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:write") {
        return Err(ApiError::Forbidden);
    }
    let invoice = InvoiceRepo::update_quote_status(&state.db, &claims.org, &id, "declined").await?;
    Ok(Json(serde_json::json!({ "data": invoice })))
}

/// POST /api/v1/quotes/:id/progress-invoice
pub async fn progress_invoice(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<ProgressInvoiceInput>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("invoices:write") {
        return Err(ApiError::Forbidden);
    }
    let invoice = InvoiceRepo::progress_invoice(
        &state.db,
        &claims.org,
        &id,
        body.pct_bps,
        body.invoice_date,
        body.due_date,
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": invoice })),
    ))
}

/// POST /api/v1/quotes/:id/expire
pub async fn expire_quote(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:write") {
        return Err(ApiError::Forbidden);
    }
    let invoice = InvoiceRepo::update_quote_status(&state.db, &claims.org, &id, "expired").await?;
    Ok(Json(serde_json::json!({ "data": invoice })))
}

/// POST /api/v1/invoices/cash-sale
/// Creates an invoice and a full payment in one atomic step (no AR created).
pub async fn create_cash_sale(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateCashSale>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("invoices:write") {
        return Err(ApiError::Forbidden);
    }

    let sale_date = body.date;
    let payment_method = body.payment_method.clone();
    let invoice_input = CreateInvoice {
        contact_id: body.contact_id,
        invoice_type: InvoiceType::Invoice,
        date: sale_date,
        due_date: sale_date,
        currency: body.currency,
        exchange_rate: body.exchange_rate,
        notes: body.notes,
        global_discount_pct: 0,
        lines: body.lines,
    };

    let invoice = InvoiceRepo::create(&state.db, &claims.org, invoice_input).await?;

    let payment_input = CreatePayment {
        amount: invoice.total(),
        payment_date: sale_date,
        method: payment_method,
        reference: None,
        notes: None,
        exchange_rate: None,
    };

    let payment = PaymentRepo::create(&state.db, &claims.org, &invoice.id, payment_input).await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": { "invoice": invoice, "payment": payment } })),
    ))
}

/// POST /api/v1/invoices/from-expenses
/// Wraps a set of approved billable expenses into a new invoice and marks them billed.
pub async fn create_from_expenses(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<BillableExpenseRef>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("invoices:write") {
        return Err(ApiError::Forbidden);
    }
    if body.expense_ids.is_empty() {
        return Err(ApiError::BadRequest("expense_ids must not be empty".into()));
    }

    // Fetch and validate every expense.
    let mut lines: Vec<CreateInvoiceLine> = Vec::with_capacity(body.expense_ids.len());
    let mut expense_uuids: Vec<Uuid> = Vec::with_capacity(body.expense_ids.len());

    for eid in &body.expense_ids {
        let exp = ExpenseRepo::get_by_id(&state.db, &claims.org, eid).await?;

        if !exp.is_billable {
            return Err(ApiError::BadRequest(format!(
                "expense {eid} is not marked as billable"
            )));
        }
        if exp.billed_invoice_id.is_some() {
            return Err(ApiError::BadRequest(format!(
                "expense {eid} has already been billed"
            )));
        }
        if exp.billable_contact_id.as_deref() != Some(&body.contact_id) {
            return Err(ApiError::BadRequest(format!(
                "expense {eid} is not billable to contact {}",
                body.contact_id
            )));
        }

        lines.push(CreateInvoiceLine {
            description: exp.description.clone(),
            quantity: 100,
            unit_price: exp.amount,
            account_id: exp.account_id.clone(),
            tax_rate: None,
            discount_pct: 0,
            product_id: None,
            variant_id: None,
        });

        expense_uuids.push(
            Uuid::parse_str(eid)
                .map_err(|_| ApiError::BadRequest(format!("invalid UUID: {eid}")))?,
        );
    }

    let invoice_input = CreateInvoice {
        contact_id: body.contact_id,
        invoice_type: InvoiceType::Invoice,
        date: body.invoice_date,
        due_date: body.due_date.unwrap_or(body.invoice_date),
        currency: None,
        exchange_rate: None,
        notes: None,
        global_discount_pct: 0,
        lines,
    };

    let invoice = InvoiceRepo::create(&state.db, &claims.org, invoice_input).await?;

    ExpenseRepo::mark_billed(
        &state.db,
        &claims.org,
        &expense_uuids,
        Uuid::parse_str(&invoice.id).expect("invoice id is valid UUID"),
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": invoice })),
    ))
}
