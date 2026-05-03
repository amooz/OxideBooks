use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateDepartment, UpdateDepartment};
use oxidebooks_db::repos::DepartmentRepo;
use serde::Deserialize;
use time::macros::format_description;
use time::Date;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

fn parse_date(s: &str) -> Result<Date, ApiError> {
    let fmt = format_description!("[year]-[month]-[day]");
    Date::parse(s, fmt)
        .map_err(|_| ApiError::BadRequest(format!("invalid date '{s}'; expected YYYY-MM-DD")))
}

#[derive(Deserialize)]
pub struct DeptPlQuery {
    pub from: String,
    pub to: String,
}

/// GET /api/v1/departments
pub async fn list_departments(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let depts = DepartmentRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": depts })))
}

/// POST /api/v1/departments
pub async fn create_department(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateDepartment>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let dept = DepartmentRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": dept })),
    ))
}

/// PATCH /api/v1/departments/:id
pub async fn update_department(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateDepartment>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let dept = DepartmentRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": dept })))
}

/// DELETE /api/v1/departments/:id
pub async fn delete_department(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    DepartmentRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/reports/department-pl?department_id=&from=&to=
pub async fn department_pl(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<DeptPlQuery>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let from = parse_date(&q.from)?;
    let to = parse_date(&q.to)?;
    if from > to {
        return Err(ApiError::BadRequest(
            "'from' must be on or before 'to'".into(),
        ));
    }
    let report = DepartmentRepo::department_pl(&state.db, &claims.org, &id, from, to).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}
