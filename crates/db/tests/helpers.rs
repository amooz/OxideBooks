/// Shared test fixtures for DB integration tests.
///
/// Each `#[sqlx::test]` receives its own isolated database with migrations
/// already applied — no shared state between tests.
use oxidebooks_core::models::{
    AccountType, CreateAccount, CreateOrganization, CreateJournalEntry, CreateJournalLine,
};
use oxidebooks_db::repos::{AccountRepo, OrganizationRepo, TransactionRepo, UserRepo};
use oxidebooks_db::repos::users::CreateUser;
use sqlx::PgPool;
use time::{Date, Month};

pub async fn seed_org(pool: &PgPool) -> oxidebooks_core::models::Organization {
    OrganizationRepo::create(
        pool,
        CreateOrganization {
            name: "Test Org".to_string(),
            currency: "USD".to_string(),
            fiscal_year_start: None,
        },
    )
    .await
    .expect("seed org")
}

pub async fn seed_user(pool: &PgPool, org_id: &str) -> oxidebooks_db::repos::users::User {
    UserRepo::create(
        pool,
        CreateUser {
            organization_id: org_id.to_string(),
            email: "test@example.com".to_string(),
            password_hash: "$argon2id$stub".to_string(),
            name: "Test User".to_string(),
            role: "owner".to_string(),
        },
    )
    .await
    .expect("seed user")
}

pub async fn seed_account(
    pool: &PgPool,
    org_id: &str,
    code: &str,
    name: &str,
    account_type: AccountType,
) -> oxidebooks_core::models::Account {
    AccountRepo::create(
        pool,
        org_id,
        CreateAccount {
            code: code.to_string(),
            name: name.to_string(),
            account_type,
            parent_id: None,
            description: None,
        },
    )
    .await
    .expect("seed account")
}

pub fn jan_15() -> Date {
    Date::from_calendar_date(2025, Month::January, 15).unwrap()
}

pub async fn post_simple_entry(
    pool: &PgPool,
    org_id: &str,
    user_id: &str,
    debit_account_id: &str,
    credit_account_id: &str,
    amount: i64,
) -> oxidebooks_core::models::JournalEntry {
    TransactionRepo::create(
        pool,
        org_id,
        user_id,
        CreateJournalEntry {
            date: jan_15(),
            reference: None,
            description: "Test entry".to_string(),
            lines: vec![
                CreateJournalLine {
                    account_id: debit_account_id.to_string(),
                    description: None,
                    debit: amount,
                    credit: 0,
                },
                CreateJournalLine {
                    account_id: credit_account_id.to_string(),
                    description: None,
                    debit: 0,
                    credit: amount,
                },
            ],
        },
    )
    .await
    .expect("post entry")
}
