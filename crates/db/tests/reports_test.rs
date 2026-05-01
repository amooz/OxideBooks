mod helpers;

use oxidebooks_core::models::AccountType;
use oxidebooks_db::repos::ReportRepo;

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn trial_balance_is_balanced_after_posting(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    let user = helpers::seed_user(&pool, &org.id).await;
    let cash = helpers::seed_account(&pool, &org.id, "1000", "Cash", AccountType::Asset).await;
    let revenue = helpers::seed_account(&pool, &org.id, "4000", "Revenue", AccountType::Revenue).await;

    helpers::post_simple_entry(&pool, &org.id, &user.id, &cash.id, &revenue.id, 10_000).await;

    let tb = ReportRepo::trial_balance(&pool, &org.id).await.unwrap();
    assert!(tb.is_balanced(), "trial balance must be balanced after a valid posting");
    assert_eq!(tb.total_debits, 10_000);
    assert_eq!(tb.total_credits, 10_000);
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn trial_balance_empty_org_has_zero_totals(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;

    let tb = ReportRepo::trial_balance(&pool, &org.id).await.unwrap();
    assert!(tb.is_balanced());
    assert_eq!(tb.total_debits, 0);
    assert_eq!(tb.total_credits, 0);
    assert!(tb.accounts.is_empty());
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn trial_balance_includes_accounts_with_zero_activity(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    helpers::seed_account(&pool, &org.id, "1000", "Cash", AccountType::Asset).await;
    helpers::seed_account(&pool, &org.id, "4000", "Revenue", AccountType::Revenue).await;

    let tb = ReportRepo::trial_balance(&pool, &org.id).await.unwrap();
    // Both accounts appear even though no entries have been posted.
    assert_eq!(tb.accounts.len(), 2);
    assert!(tb.accounts.iter().all(|a| a.debit_total == 0 && a.credit_total == 0));
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn trial_balance_account_balances_are_correct(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    let user = helpers::seed_user(&pool, &org.id).await;
    let cash = helpers::seed_account(&pool, &org.id, "1000", "Cash", AccountType::Asset).await;
    let revenue = helpers::seed_account(&pool, &org.id, "4000", "Revenue", AccountType::Revenue).await;

    helpers::post_simple_entry(&pool, &org.id, &user.id, &cash.id, &revenue.id, 10_000).await;
    helpers::post_simple_entry(&pool, &org.id, &user.id, &cash.id, &revenue.id, 5_000).await;

    let tb = ReportRepo::trial_balance(&pool, &org.id).await.unwrap();

    let cash_bal = tb.accounts.iter().find(|a| a.account_code == "1000").unwrap();
    assert_eq!(cash_bal.debit_total, 15_000);
    assert_eq!(cash_bal.credit_total, 0);
    assert_eq!(cash_bal.balance(), 15_000); // asset: debit-normal

    let rev_bal = tb.accounts.iter().find(|a| a.account_code == "4000").unwrap();
    assert_eq!(rev_bal.debit_total, 0);
    assert_eq!(rev_bal.credit_total, 15_000);
    assert_eq!(rev_bal.balance(), 15_000); // revenue: credit-normal
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn trial_balance_isolated_to_org(pool: sqlx::PgPool) {
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

    let user1 = helpers::seed_user(&pool, &org1.id).await;
    let cash1 = helpers::seed_account(&pool, &org1.id, "1000", "Cash", AccountType::Asset).await;
    let rev1 = helpers::seed_account(&pool, &org1.id, "4000", "Revenue", AccountType::Revenue).await;
    helpers::post_simple_entry(&pool, &org1.id, &user1.id, &cash1.id, &rev1.id, 50_000).await;

    // org2 has no entries — its trial balance should be zero.
    let tb2 = ReportRepo::trial_balance(&pool, &org2.id).await.unwrap();
    assert_eq!(tb2.total_debits, 0);
    assert_eq!(tb2.total_credits, 0);
}
