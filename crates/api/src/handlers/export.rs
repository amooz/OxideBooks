use axum::{
    extract::{Extension, Query, State},
    http::{header, StatusCode},
    response::Response,
};
use oxidebooks_core::pagination::PageParams;
use oxidebooks_db::repos::{AccountRepo, ExpenseRepo, InvoiceRepo, ReportRepo, TransactionRepo};
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    pub from: Option<String>,
    pub to: Option<String>,
}

fn csv_response(filename: &str, body: Vec<u8>) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .body(axum::body::Body::from(body))
        .expect("static response is valid")
}

fn csv_err(e: csv::Error) -> ApiError {
    ApiError::Internal(anyhow::anyhow!("{e}"))
}

fn finish(wtr: csv::Writer<Vec<u8>>) -> ApiResult<Vec<u8>> {
    wtr.into_inner()
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))
}

const BIG_PAGE: PageParams = PageParams {
    limit: 2000,
    after: None,
};

pub async fn export_invoices(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Response> {
    use oxidebooks_core::models::InvoiceFilters;
    let (invoices, _) = InvoiceRepo::list(
        &state.db,
        &claims.org,
        &BIG_PAGE,
        &InvoiceFilters::default(),
    )
    .await?;
    let mut wtr = csv::Writer::from_writer(vec![]);
    wtr.write_record([
        "id",
        "number",
        "contact_id",
        "status",
        "type",
        "total",
        "currency",
        "date",
        "due_date",
    ])
    .map_err(csv_err)?;
    for inv in &invoices {
        wtr.write_record([
            &inv.id,
            &inv.invoice_number,
            &inv.contact_id,
            &inv.status.to_string(),
            &inv.invoice_type.to_string(),
            &inv.total().to_string(),
            &inv.currency,
            &inv.date.to_string(),
            &inv.due_date.to_string(),
        ])
        .map_err(csv_err)?;
    }
    Ok(csv_response("invoices.csv", finish(wtr)?))
}

pub async fn export_expenses(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Response> {
    let (expenses, _) = ExpenseRepo::list(&state.db, &claims.org, &BIG_PAGE, None, None).await?;
    let mut wtr = csv::Writer::from_writer(vec![]);
    wtr.write_record([
        "id",
        "date",
        "amount",
        "currency",
        "description",
        "status",
        "category",
    ])
    .map_err(csv_err)?;
    for exp in &expenses {
        wtr.write_record([
            &exp.id,
            &exp.expense_date.to_string(),
            &exp.amount.to_string(),
            &exp.currency,
            &exp.description,
            &exp.status.to_string(),
            &exp.category,
        ])
        .map_err(csv_err)?;
    }
    Ok(csv_response("expenses.csv", finish(wtr)?))
}

pub async fn export_transactions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Response> {
    let (entries, _) = TransactionRepo::list(&state.db, &claims.org, &BIG_PAGE).await?;
    let mut wtr = csv::Writer::from_writer(vec![]);
    wtr.write_record(["id", "date", "reference", "description", "status"])
        .map_err(csv_err)?;
    for je in &entries {
        wtr.write_record([
            &je.id,
            &je.date.to_string(),
            je.reference.as_deref().unwrap_or(""),
            &je.description,
            &je.status.to_string(),
        ])
        .map_err(csv_err)?;
    }
    Ok(csv_response("transactions.csv", finish(wtr)?))
}

pub async fn export_profit_loss(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<ExportQuery>,
) -> ApiResult<Response> {
    use time::{format_description, Date, Month, OffsetDateTime};
    let fmt = format_description::parse("[year]-[month]-[day]").expect("static fmt");
    let from = q
        .from
        .as_deref()
        .and_then(|s| Date::parse(s, &fmt).ok())
        .unwrap_or_else(|| {
            let now = OffsetDateTime::now_utc();
            Date::from_calendar_date(now.year(), Month::January, 1).unwrap()
        });
    let to =
        q.to.as_deref()
            .and_then(|s| Date::parse(s, &fmt).ok())
            .unwrap_or_else(|| OffsetDateTime::now_utc().date());

    let report = ReportRepo::profit_loss(&state.db, &claims.org, from, to).await?;
    let mut wtr = csv::Writer::from_writer(vec![]);
    wtr.write_record(["section", "account_id", "account_name", "amount"])
        .map_err(csv_err)?;
    for line in &report.revenue.accounts {
        wtr.write_record([
            "revenue",
            &line.account_id,
            &line.account_name,
            &line.amount.to_string(),
        ])
        .map_err(csv_err)?;
    }
    wtr.write_record(["revenue", "", "TOTAL", &report.revenue.total.to_string()])
        .map_err(csv_err)?;
    for line in &report.expenses.accounts {
        wtr.write_record([
            "expenses",
            &line.account_id,
            &line.account_name,
            &line.amount.to_string(),
        ])
        .map_err(csv_err)?;
    }
    wtr.write_record(["expenses", "", "TOTAL", &report.expenses.total.to_string()])
        .map_err(csv_err)?;
    wtr.write_record(["", "", "NET INCOME", &report.net_income.to_string()])
        .map_err(csv_err)?;
    Ok(csv_response("profit_loss.csv", finish(wtr)?))
}

pub async fn export_trial_balance(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Response> {
    let tb = ReportRepo::trial_balance(&state.db, &claims.org).await?;
    let mut wtr = csv::Writer::from_writer(vec![]);
    wtr.write_record([
        "account_id",
        "account_code",
        "account_name",
        "debit",
        "credit",
    ])
    .map_err(csv_err)?;
    for acct in &tb.accounts {
        wtr.write_record([
            &acct.account_id,
            &acct.account_code,
            &acct.account_name,
            &acct.debit_total.to_string(),
            &acct.credit_total.to_string(),
        ])
        .map_err(csv_err)?;
    }
    wtr.write_record([
        "",
        "",
        "TOTALS",
        &tb.total_debits.to_string(),
        &tb.total_credits.to_string(),
    ])
    .map_err(csv_err)?;
    Ok(csv_response("trial_balance.csv", finish(wtr)?))
}

/// GET /api/v1/export/accounts
pub async fn export_accounts(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Response> {
    if !claims.has("invoices:read") {
        return Err(ApiError::Forbidden);
    }
    let (accounts, _) = AccountRepo::list(&state.db, &claims.org, &BIG_PAGE).await?;
    let mut wtr = csv::Writer::from_writer(vec![]);
    wtr.write_record(["code", "name", "account_type", "description", "is_active"])
        .map_err(csv_err)?;
    for acct in &accounts {
        wtr.write_record([
            &acct.code,
            &acct.name,
            &acct.account_type.to_string(),
            acct.description.as_deref().unwrap_or(""),
            &acct.is_active.to_string(),
        ])
        .map_err(csv_err)?;
    }
    Ok(csv_response("accounts.csv", finish(wtr)?))
}
