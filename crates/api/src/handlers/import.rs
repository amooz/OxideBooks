use axum::{
    extract::{Extension, State},
    Json,
};
use oxidebooks_core::models::{
    AccountCsvRow, AccountType, ContactCsvRow, CreateAccount, CreateContact, ImportError,
    ImportResult,
};
use oxidebooks_db::repos::{AccountRepo, ContactRepo};
use std::str::FromStr;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

pub async fn import_contacts_csv(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    body: String,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(body.as_bytes());

    let mut rows_ok = 0usize;
    let mut errors: Vec<ImportError> = vec![];

    for (idx, result) in rdr.deserialize::<ContactCsvRow>().enumerate() {
        let row_num = idx + 2; // 1-indexed, header is row 1
        match result {
            Err(e) => errors.push(ImportError {
                row: row_num,
                message: e.to_string(),
            }),
            Ok(row) => {
                let contact_type = row.contact_type.and_then(|t| {
                    serde_json::from_str::<oxidebooks_core::models::ContactType>(&format!(
                        "\"{t}\""
                    ))
                    .ok()
                });

                let create = CreateContact {
                    name: row.name,
                    email: row.email,
                    phone: row.phone,
                    contact_type,
                    address: None,
                    tax_number: None,
                    currency: row.currency,
                };

                match ContactRepo::create(&state.db, &claims.org, create).await {
                    Ok(_) => rows_ok += 1,
                    Err(e) => errors.push(ImportError {
                        row: row_num,
                        message: e.to_string(),
                    }),
                }
            }
        }
    }

    let result = ImportResult {
        rows_ok,
        rows_failed: errors.len(),
        errors,
    };
    Ok(Json(serde_json::json!(result)))
}

/// POST /api/v1/import/accounts — import chart of accounts from CSV
pub async fn import_accounts_csv(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    body: String,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(body.as_bytes());

    let mut rows_ok = 0usize;
    let mut errors: Vec<ImportError> = vec![];

    for (idx, result) in rdr.deserialize::<AccountCsvRow>().enumerate() {
        let row_num = idx + 2;
        match result {
            Err(e) => errors.push(ImportError {
                row: row_num,
                message: e.to_string(),
            }),
            Ok(row) => {
                let account_type = match AccountType::from_str(&row.account_type) {
                    Ok(t) => t,
                    Err(_) => {
                        errors.push(ImportError {
                            row: row_num,
                            message: format!("unknown account_type '{}'", row.account_type),
                        });
                        continue;
                    }
                };
                let create = CreateAccount {
                    code: row.code,
                    name: row.name,
                    account_type,
                    parent_id: None,
                    description: row.description,
                };
                match AccountRepo::create(&state.db, &claims.org, create).await {
                    Ok(_) => rows_ok += 1,
                    Err(e) => errors.push(ImportError {
                        row: row_num,
                        message: e.to_string(),
                    }),
                }
            }
        }
    }

    let result = ImportResult {
        rows_ok,
        rows_failed: errors.len(),
        errors,
    };
    Ok(Json(serde_json::json!(result)))
}
