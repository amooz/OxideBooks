mod handler_helpers;

use axum::body::Body;
use http::{Request, StatusCode};
use oxidebooks_core::models::CreateOrganization;
use oxidebooks_db::repos::users::CreateUser;
use oxidebooks_db::repos::{OrganizationRepo, UserRepo};

use handler_helpers::{build_app, json_body, mint_test_token, send};

async fn seed_org_user(pool: &sqlx::PgPool) -> (String, String) {
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

    (org.id, user.id)
}

// ── GET /api/v1/permissions ───────────────────────────────────────────────────

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn list_permissions_requires_roles_read(pool: sqlx::PgPool) {
    let (org_id, user_id) = seed_org_user(&pool).await;
    let app = build_app(pool);
    let token = mint_test_token(&user_id, &org_id, "viewer", vec!["accounts:read"]);

    let req = Request::builder()
        .uri("/api/v1/permissions")
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let resp = send(app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn list_permissions_returns_all_with_roles_read(pool: sqlx::PgPool) {
    let (org_id, user_id) = seed_org_user(&pool).await;
    let app = build_app(pool);
    let token = mint_test_token(&user_id, &org_id, "admin", vec!["roles:read"]);

    let req = Request::builder()
        .uri("/api/v1/permissions")
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let resp = send(app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = json_body(resp).await;
    let perms = body["data"].as_array().unwrap();
    assert_eq!(perms.len(), 15);
}

// ── GET /api/v1/roles ─────────────────────────────────────────────────────────

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn list_roles_returns_system_roles(pool: sqlx::PgPool) {
    let (org_id, user_id) = seed_org_user(&pool).await;
    let app = build_app(pool);
    let token = mint_test_token(&user_id, &org_id, "owner", vec!["roles:read"]);

    let req = Request::builder()
        .uri("/api/v1/roles")
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let resp = send(app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = json_body(resp).await;
    let roles = body["data"].as_array().unwrap();
    let names: Vec<&str> = roles.iter().map(|r| r["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"owner"));
    assert!(names.contains(&"viewer"));
}

// ── POST /api/v1/roles ────────────────────────────────────────────────────────

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn create_role_returns_201(pool: sqlx::PgPool) {
    let (org_id, user_id) = seed_org_user(&pool).await;
    let app = build_app(pool);
    let token = mint_test_token(&user_id, &org_id, "owner", vec!["roles:write"]);

    let body = serde_json::json!({ "name": "billing-manager" });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/roles")
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let resp = send(app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp_body = json_body(resp).await;
    assert_eq!(resp_body["data"]["name"], "billing-manager");
    assert!(!resp_body["data"]["is_system"].as_bool().unwrap());
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn create_role_returns_403_without_roles_write(pool: sqlx::PgPool) {
    let (org_id, user_id) = seed_org_user(&pool).await;
    let app = build_app(pool);
    let token = mint_test_token(&user_id, &org_id, "admin", vec!["roles:read"]);

    let body = serde_json::json!({ "name": "custom-role" });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/roles")
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let resp = send(app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn create_role_rejects_empty_name(pool: sqlx::PgPool) {
    let (org_id, user_id) = seed_org_user(&pool).await;
    let app = build_app(pool);
    let token = mint_test_token(&user_id, &org_id, "owner", vec!["roles:write"]);

    let body = serde_json::json!({ "name": "   " });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/roles")
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let resp = send(app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ── POST /api/v1/roles/:id/permissions ───────────────────────────────────────

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn assign_permission_to_custom_role_returns_204(pool: sqlx::PgPool) {
    let (org_id, user_id) = seed_org_user(&pool).await;

    // Create a custom role
    let role = oxidebooks_db::repos::RoleRepo::create(&pool, &org_id, "reporter")
        .await
        .unwrap();

    let app = build_app(pool);
    let token = mint_test_token(&user_id, &org_id, "owner", vec!["roles:write"]);
    let body = serde_json::json!({ "permission": "reports:read" });

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/roles/{}/permissions", role.id))
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let resp = send(app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}
