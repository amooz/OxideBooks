use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{
    CreateRecurringSchedule, Frequency, InvoiceStatus, RecurringSchedule, UpdateInvoice,
    UpdateRecurringSchedule,
};
use oxidebooks_db::repos::{InvoiceRepo, RecurringRepo};
use time::Date;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

fn advance_date(date: Date, frequency: &Frequency, interval: i32) -> Option<Date> {
    match frequency {
        Frequency::Weekly => Some(date + time::Duration::weeks(interval as i64)),
        Frequency::Monthly | Frequency::Quarterly => {
            let step = match frequency {
                Frequency::Quarterly => interval * 3,
                _ => interval,
            };
            let total = date.month() as i32 + step;
            let year_add = (total - 1) / 12;
            let month = time::Month::try_from(((total - 1) % 12 + 1) as u8).ok()?;
            let year = date.year() + year_add;
            Date::from_calendar_date(year, month, date.day())
                .or_else(|_| {
                    // Clamp to last day of month
                    let last = time::util::days_in_month(month, year);
                    Date::from_calendar_date(year, month, last)
                })
                .ok()
        }
        Frequency::Yearly => {
            Date::from_calendar_date(date.year() + interval, date.month(), date.day())
                .or_else(|_| Date::from_calendar_date(date.year() + interval, date.month(), 28))
                .ok()
        }
    }
}

async fn run_schedule_inner(
    state: &AppState,
    org_id: &str,
    schedule: &RecurringSchedule,
) -> ApiResult<String> {
    let mut input =
        serde_json::from_value::<oxidebooks_core::models::CreateInvoice>(schedule.template.clone())
            .map_err(|e| ApiError::BadRequest(format!("invalid template: {e}")))?;

    let today = time::OffsetDateTime::now_utc().date();
    input.date = today;

    let invoice = InvoiceRepo::create(&state.db, org_id, input).await?;

    if schedule.auto_send {
        let _ = InvoiceRepo::update(
            &state.db,
            org_id,
            &invoice.id,
            UpdateInvoice {
                status: Some(InvoiceStatus::Sent),
                due_date: None,
                notes: None,
            },
        )
        .await;
    }

    if let Some(next_due) = advance_date(
        schedule.next_due_date,
        &schedule.frequency,
        schedule.interval_count,
    ) {
        let past_end = schedule.end_date.is_some_and(|end| next_due > end);
        if past_end {
            let _ = RecurringRepo::update(
                &state.db,
                org_id,
                &schedule.id,
                UpdateRecurringSchedule {
                    is_active: Some(false),
                    next_due_date: None,
                    end_date: None,
                    auto_send: None,
                },
            )
            .await;
        } else {
            RecurringRepo::advance(&state.db, &schedule.id, next_due).await?;
        }
    }

    Ok(invoice.id)
}

/// GET /api/v1/recurring-schedules
pub async fn list_schedules(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:read") {
        return Err(ApiError::Forbidden);
    }
    let schedules = RecurringRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": schedules })))
}

/// GET /api/v1/recurring-schedules/:id
pub async fn get_schedule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:read") {
        return Err(ApiError::Forbidden);
    }
    let schedule = RecurringRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": schedule })))
}

/// POST /api/v1/recurring-schedules
pub async fn create_schedule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateRecurringSchedule>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("invoices:write") {
        return Err(ApiError::Forbidden);
    }
    let schedule = RecurringRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": schedule })),
    ))
}

/// PATCH /api/v1/recurring-schedules/:id
pub async fn update_schedule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateRecurringSchedule>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:write") {
        return Err(ApiError::Forbidden);
    }
    let schedule = RecurringRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": schedule })))
}

/// DELETE /api/v1/recurring-schedules/:id
pub async fn delete_schedule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.has("invoices:write") {
        return Err(ApiError::Forbidden);
    }
    RecurringRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/recurring-schedules/run-due
pub async fn run_due_schedules(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let today = time::OffsetDateTime::now_utc().date();
    let all_due = RecurringRepo::list_due(&state.db, today).await?;
    let mut generated = Vec::new();
    let mut skipped = Vec::new();
    for schedule in all_due {
        if schedule.organization_id != claims.org {
            continue;
        }
        match run_schedule_inner(&state, &claims.org, &schedule).await {
            Ok(id) => generated.push(id),
            Err(_) => skipped.push(schedule.id),
        }
    }
    Ok(Json(
        serde_json::json!({ "generated": generated, "skipped": skipped }),
    ))
}

/// POST /api/v1/recurring-schedules/:id/run
pub async fn run_schedule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let schedule = RecurringRepo::get_by_id(&state.db, &claims.org, &id).await?;
    let invoice_id = run_schedule_inner(&state, &claims.org, &schedule).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "invoice_id": invoice_id })),
    ))
}
