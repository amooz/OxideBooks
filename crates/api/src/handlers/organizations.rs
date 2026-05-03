use axum::{
    extract::{Extension, State},
    Json,
};
use oxidebooks_core::models::UpdateOrganization;
use oxidebooks_db::repos::organizations::OrganizationRepo;
use tracing::info;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/organizations/me
pub async fn get_org(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    let org = OrganizationRepo::get_by_id_str(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": org })))
}

/// PATCH /api/v1/organizations/me
pub async fn update_org(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<UpdateOrganization>,
) -> ApiResult<Json<serde_json::Value>> {
    if claims.role != "owner" {
        return Err(ApiError::Forbidden);
    }

    if let Some(fys) = body.fiscal_year_start {
        if !(1..=12).contains(&fys) {
            return Err(ApiError::BadRequest(
                "fiscal_year_start must be between 1 and 12".into(),
            ));
        }
    }

    let org = OrganizationRepo::update(&state.db, &claims.org, body).await?;
    info!(org_id = %claims.org, "🏢 organization settings updated");
    Ok(Json(serde_json::json!({ "data": org })))
}
