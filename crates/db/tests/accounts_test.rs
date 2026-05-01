mod helpers;

use oxidebooks_core::models::{AccountType, CreateAccount, UpdateAccount};
use oxidebooks_db::repos::AccountRepo;

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn create_account_returns_correct_fields(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;

    let account = AccountRepo::create(
        &pool,
        &org.id,
        CreateAccount {
            code: "1000".to_string(),
            name: "Cash".to_string(),
            account_type: AccountType::Asset,
            parent_id: None,
            description: Some("Petty cash".to_string()),
        },
    )
    .await
    .unwrap();

    assert_eq!(account.code, "1000");
    assert_eq!(account.name, "Cash");
    assert_eq!(account.account_type, AccountType::Asset);
    assert_eq!(account.description.as_deref(), Some("Petty cash"));
    assert_eq!(account.organization_id, org.id);
    assert!(account.is_active);
    assert!(account.parent_id.is_none());
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn list_accounts_returns_all_for_org(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;

    helpers::seed_account(&pool, &org.id, "1000", "Cash", AccountType::Asset).await;
    helpers::seed_account(&pool, &org.id, "2000", "Accounts Payable", AccountType::Liability).await;
    helpers::seed_account(&pool, &org.id, "4000", "Revenue", AccountType::Revenue).await;

    let accounts = AccountRepo::list(&pool, &org.id).await.unwrap();
    assert_eq!(accounts.len(), 3);
    // Should be sorted by code
    assert_eq!(accounts[0].code, "1000");
    assert_eq!(accounts[1].code, "2000");
    assert_eq!(accounts[2].code, "4000");
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn list_accounts_isolated_to_org(pool: sqlx::PgPool) {
    let org1 = helpers::seed_org(&pool).await;
    let org2 = oxidebooks_db::repos::OrganizationRepo::create(
        &pool,
        oxidebooks_core::models::CreateOrganization {
            name: "Other Org".to_string(),
            currency: "USD".to_string(),
            fiscal_year_start: None,
        },
    )
    .await
    .unwrap();

    helpers::seed_account(&pool, &org1.id, "1000", "Cash", AccountType::Asset).await;
    helpers::seed_account(&pool, &org2.id, "1000", "Cash", AccountType::Asset).await;

    let accounts = AccountRepo::list(&pool, &org1.id).await.unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].organization_id, org1.id);
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn get_account_by_id(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    let created = helpers::seed_account(&pool, &org.id, "1000", "Cash", AccountType::Asset).await;

    let fetched = AccountRepo::get_by_id(&pool, &org.id, &created.id).await.unwrap();
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.code, "1000");
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn get_account_not_found(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    let missing_id = uuid::Uuid::new_v4().to_string();

    let result = AccountRepo::get_by_id(&pool, &org.id, &missing_id).await;
    assert!(matches!(result, Err(oxidebooks_db::DbError::NotFound)));
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn update_account_fields(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    let account = helpers::seed_account(&pool, &org.id, "1000", "Cash", AccountType::Asset).await;

    let updated = AccountRepo::update(
        &pool,
        &org.id,
        &account.id,
        UpdateAccount {
            name: Some("Petty Cash".to_string()),
            code: None,
            description: Some("Updated description".to_string()),
            is_active: None,
        },
    )
    .await
    .unwrap();

    assert_eq!(updated.name, "Petty Cash");
    assert_eq!(updated.code, "1000"); // unchanged
    assert_eq!(updated.description.as_deref(), Some("Updated description"));
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn update_account_deactivate(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    let account = helpers::seed_account(&pool, &org.id, "1000", "Cash", AccountType::Asset).await;

    let updated = AccountRepo::update(
        &pool,
        &org.id,
        &account.id,
        UpdateAccount {
            is_active: Some(false),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    assert!(!updated.is_active);
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn delete_account(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    let account = helpers::seed_account(&pool, &org.id, "1000", "Cash", AccountType::Asset).await;

    AccountRepo::delete(&pool, &org.id, &account.id).await.unwrap();

    let result = AccountRepo::get_by_id(&pool, &org.id, &account.id).await;
    assert!(matches!(result, Err(oxidebooks_db::DbError::NotFound)));
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn delete_nonexistent_account_returns_not_found(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    let missing_id = uuid::Uuid::new_v4().to_string();

    let result = AccountRepo::delete(&pool, &org.id, &missing_id).await;
    assert!(matches!(result, Err(oxidebooks_db::DbError::NotFound)));
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn duplicate_account_code_returns_conflict(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    helpers::seed_account(&pool, &org.id, "1000", "Cash", AccountType::Asset).await;

    let result = AccountRepo::create(
        &pool,
        &org.id,
        CreateAccount {
            code: "1000".to_string(),
            name: "Another Cash".to_string(),
            account_type: AccountType::Asset,
            parent_id: None,
            description: None,
        },
    )
    .await;

    assert!(matches!(result, Err(oxidebooks_db::DbError::Conflict(_))));
}
