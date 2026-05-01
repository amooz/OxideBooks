mod handler_helpers;

use axum::body::Body;
use http::{Request, StatusCode};
use oxidebooks_core::models::{AccountType, CreateAccount, CreateOrganization};
use oxidebooks_db::repos::users::CreateUser;
use oxidebooks_db::repos::{AccountRepo, OrganizationRepo, UserRepo};

use handler_helpers::{build_app, json_body, mint_test_token, send};

// ── Fixture helpers ───────────────────────────────────────────────────────────

async fn seed_org_and_account(pool: &sqlx::PgPool) -> (String, String, String) {
    let org = OrganizationRepo::create(
        pool,
        CreateOrganization {
            name: "Test Org".into(),
            currency: "USD".into(),
            fiscal_year_start: None,
        },
    )
    .await
    .unwrap();

    let user = UserRepo::create(
        pool,
        CreateUser {
            organization_id: org.id.clone(),
            email: "owner@example.com".into(),
            password_hash: "$argon2id$stub".into(),
            name: "Owner".into(),
            role: "owner".into(),
        },
    )
    .await
    .unwrap();

    let account = AccountRepo::create(
        pool,
        &org.id,
        CreateAccount {
            code: "1000".into(),
            name: "Cash".into(),
            account_type: AccountType::Asset,
            parent_id: None,
            description: None,
        },
    )
    .await
    .unwrap();

    (org.id, user.id, account.id)
}

// ── GET /api/v1/accounts ──────────────────────────────────────────────────────

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn list_accounts_returns_200_for_accounts_read(pool: sqlx::PgPool) {
    let (org_id, user_id, _) = seed_org_and_account(&pool).await;
    let app = build_app(pool);
    let token = mint_test_token(&user_id, &org_id, "viewer", vec!["accounts:read"]);

    let req = Request::builder()
        .uri("/api/v1/accounts")
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let resp = send(app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = json_body(resp).await;
    assert!(body["data"].is_array());
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn list_accounts_returns_403_without_accounts_read(pool: sqlx::PgPool) {
    let (org_id, user_id, _) = seed_org_and_account(&pool).await;
    let app = build_app(pool);
    // token with NO accounts:read
    let token = mint_test_token(&user_id, &org_id, "limited", vec!["invoices:read"]);

    let req = Request::builder()
        .uri("/api/v1/accounts")
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let resp = send(app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn list_accounts_returns_401_without_token(pool: sqlx::PgPool) {
    let app = build_app(pool);

    let req = Request::builder()
        .uri("/api/v1/accounts")
        .body(Body::empty())
        .unwrap();

    let resp = send(app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ── POST /api/v1/accounts ─────────────────────────────────────────────────────

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn create_account_returns_201_with_accounts_write(pool: sqlx::PgPool) {
    let org = OrganizationRepo::create(
        &pool,
        CreateOrganization {
            name: "Org".into(),
            currency: "USD".into(),
            fiscal_year_start: None,
        },
    )
    .await
    .unwrap();
    let user = UserRepo::create(
        &pool,
        CreateUser {
            organization_id: org.id.clone(),
            email: "acct@example.com".into(),
            password_hash: "$argon2id$stub".into(),
            name: "Accountant".into(),
            role: "accountant".into(),
        },
    )
    .await
    .unwrap();

    let app = build_app(pool);
    let token = mint_test_token(&user.id, &org.id, "accountant", vec!["accounts:write"]);
    let body = serde_json::json!({
        "code": "2000",
        "name": "Revenue",
        "account_type": "revenue"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/accounts")
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let resp = send(app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp_body = json_body(resp).await;
    assert_eq!(resp_body["data"]["code"], "2000");
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn create_account_returns_403_for_viewer(pool: sqlx::PgPool) {
    let org = OrganizationRepo::create(
        &pool,
        CreateOrganization {
            name: "Org".into(),
            currency: "USD".into(),
            fiscal_year_start: None,
        },
    )
    .await
    .unwrap();
    let user = UserRepo::create(
        &pool,
        CreateUser {
            organization_id: org.id.clone(),
            email: "viewer@example.com".into(),
            password_hash: "$argon2id$stub".into(),
            name: "Viewer".into(),
            role: "viewer".into(),
        },
    )
    .await
    .unwrap();

    let app = build_app(pool);
    // viewer has accounts:read but NOT accounts:write
    let token = mint_test_token(&user.id, &org.id, "viewer", vec!["accounts:read"]);
    let body = serde_json::json!({
        "code": "9999",
        "name": "Nope",
        "account_type": "asset"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/accounts")
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let resp = send(app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ── DELETE /api/v1/accounts/:id ───────────────────────────────────────────────

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn delete_account_returns_204_with_accounts_delete(pool: sqlx::PgPool) {
    let (org_id, user_id, account_id) = seed_org_and_account(&pool).await;
    let app = build_app(pool);
    let token = mint_test_token(&user_id, &org_id, "admin", vec!["accounts:delete"]);

    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/accounts/{account_id}"))
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let resp = send(app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn delete_account_returns_403_without_accounts_delete(pool: sqlx::PgPool) {
    let (org_id, user_id, account_id) = seed_org_and_account(&pool).await;
    let app = build_app(pool);
    // accountant can write but not delete
    let token = mint_test_token(&user_id, &org_id, "accountant", vec!["accounts:write"]);

    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/accounts/{account_id}"))
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let resp = send(app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ── GET /health ───────────────────────────────────────────────────────────────

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn health_check_is_public(pool: sqlx::PgPool) {
    let app = build_app(pool);
    let req = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();
    let resp = send(app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}
