/// Shared helpers for handler-level (function) integration tests.
///
/// These tests build a real Axum router backed by a live PostgreSQL test DB
/// (provisioned by `#[sqlx::test]`) and send HTTP requests via
/// `tower::ServiceExt::oneshot`.
use axum::{body::Body, Router};
use http::{Request, Response};
use http_body_util::BodyExt;
use jsonwebtoken::{encode, EncodingKey, Header};
use oxidebooks_api::{
    config::{AppSettings, AuthSettings, DatabaseSettings, ServerSettings, Settings},
    middleware::Claims,
    routes,
    state::AppState,
};
use serde_json::Value;
use sqlx::PgPool;
use std::sync::Arc;
use time::OffsetDateTime;

pub const TEST_JWT_SECRET: &str = "test-secret-for-handler-tests-min-32-chars";

pub fn test_settings(db_url: &str) -> Settings {
    Settings {
        server: ServerSettings {
            host: "127.0.0.1".into(),
            port: 3000,
        },
        database: DatabaseSettings {
            url: db_url.to_string(),
        },
        auth: AuthSettings {
            jwt_secret: TEST_JWT_SECRET.to_string(),
            token_expiry_hours: 24,
            refresh_expiry_days: 30,
        },
        app: AppSettings {
            registration_open: true,
            default_currency: "USD".to_string(),
        },
    }
}

pub fn build_app(pool: PgPool) -> Router {
    // The test DB URL doesn't matter for handler tests (pool is already open).
    let settings = test_settings("postgres://unused");
    let state = AppState {
        db: pool,
        config: Arc::new(settings),
    };
    routes::build(state)
}

/// Mint a JWT for use in tests without hitting the DB.
pub fn mint_test_token(user_id: &str, org_id: &str, role: &str, permissions: Vec<&str>) -> String {
    let exp = (OffsetDateTime::now_utc() + time::Duration::hours(24)).unix_timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        org: org_id.to_string(),
        role: role.to_string(),
        permissions: permissions.into_iter().map(String::from).collect(),
        exp,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(TEST_JWT_SECRET.as_bytes()),
    )
    .unwrap()
}

/// Send a request through the app and return the response.
pub async fn send(app: Router, req: Request<Body>) -> Response<Body> {
    use tower::ServiceExt;
    app.oneshot(req).await.unwrap()
}

/// Collect the response body as parsed JSON.
pub async fn json_body(resp: Response<Body>) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}
