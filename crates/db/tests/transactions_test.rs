mod helpers;

use oxidebooks_core::models::{AccountType, CreateJournalEntry, CreateJournalLine, JournalEntryStatus};
use oxidebooks_db::repos::TransactionRepo;

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn create_transaction_stores_entry_and_lines(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    let user = helpers::seed_user(&pool, &org.id).await;
    let cash = helpers::seed_account(&pool, &org.id, "1000", "Cash", AccountType::Asset).await;
    let revenue = helpers::seed_account(&pool, &org.id, "4000", "Revenue", AccountType::Revenue).await;

    let entry = helpers::post_simple_entry(&pool, &org.id, &user.id, &cash.id, &revenue.id, 10_000).await;

    assert_eq!(entry.organization_id, org.id);
    assert_eq!(entry.description, "Test entry");
    assert_eq!(entry.status, JournalEntryStatus::Posted);
    assert_eq!(entry.lines.len(), 2);

    let debit_line = entry.lines.iter().find(|l| l.debit > 0).unwrap();
    let credit_line = entry.lines.iter().find(|l| l.credit > 0).unwrap();

    assert_eq!(debit_line.account_id, cash.id);
    assert_eq!(debit_line.debit, 10_000);
    assert_eq!(credit_line.account_id, revenue.id);
    assert_eq!(credit_line.credit, 10_000);
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn create_transaction_rejects_unbalanced_entry(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    let user = helpers::seed_user(&pool, &org.id).await;
    let cash = helpers::seed_account(&pool, &org.id, "1000", "Cash", AccountType::Asset).await;
    let revenue = helpers::seed_account(&pool, &org.id, "4000", "Revenue", AccountType::Revenue).await;

    let result = TransactionRepo::create(
        &pool,
        &org.id,
        &user.id,
        CreateJournalEntry {
            date: helpers::jan_15(),
            reference: None,
            description: "Bad".to_string(),
            lines: vec![
                CreateJournalLine { account_id: cash.id.clone(), description: None, debit: 10_000, credit: 0 },
                CreateJournalLine { account_id: revenue.id.clone(), description: None, debit: 0, credit: 9_000 },
            ],
        },
    )
    .await;

    assert!(result.is_err());
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn create_transaction_rejects_single_line(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    let user = helpers::seed_user(&pool, &org.id).await;
    let cash = helpers::seed_account(&pool, &org.id, "1000", "Cash", AccountType::Asset).await;

    let result = TransactionRepo::create(
        &pool,
        &org.id,
        &user.id,
        CreateJournalEntry {
            date: helpers::jan_15(),
            reference: None,
            description: "Bad".to_string(),
            lines: vec![
                CreateJournalLine { account_id: cash.id.clone(), description: None, debit: 10_000, credit: 0 },
            ],
        },
    )
    .await;

    assert!(result.is_err());
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn get_transaction_by_id(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    let user = helpers::seed_user(&pool, &org.id).await;
    let cash = helpers::seed_account(&pool, &org.id, "1000", "Cash", AccountType::Asset).await;
    let revenue = helpers::seed_account(&pool, &org.id, "4000", "Revenue", AccountType::Revenue).await;

    let entry = helpers::post_simple_entry(&pool, &org.id, &user.id, &cash.id, &revenue.id, 5_000).await;
    let fetched = TransactionRepo::get_by_id(&pool, &org.id, &entry.id).await.unwrap();

    assert_eq!(fetched.id, entry.id);
    assert_eq!(fetched.lines.len(), 2);
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn get_transaction_not_found(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    let missing_id = uuid::Uuid::new_v4().to_string();

    let result = TransactionRepo::get_by_id(&pool, &org.id, &missing_id).await;
    assert!(matches!(result, Err(oxidebooks_db::DbError::NotFound)));
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn list_transactions_returns_newest_first(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    let user = helpers::seed_user(&pool, &org.id).await;
    let cash = helpers::seed_account(&pool, &org.id, "1000", "Cash", AccountType::Asset).await;
    let revenue = helpers::seed_account(&pool, &org.id, "4000", "Revenue", AccountType::Revenue).await;

    helpers::post_simple_entry(&pool, &org.id, &user.id, &cash.id, &revenue.id, 1_000).await;
    helpers::post_simple_entry(&pool, &org.id, &user.id, &cash.id, &revenue.id, 2_000).await;

    let entries = TransactionRepo::list(&pool, &org.id).await.unwrap();
    assert_eq!(entries.len(), 2);
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn create_multi_line_transaction(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    let user = helpers::seed_user(&pool, &org.id).await;
    let cash = helpers::seed_account(&pool, &org.id, "1000", "Cash", AccountType::Asset).await;
    let bank = helpers::seed_account(&pool, &org.id, "1010", "Bank", AccountType::Asset).await;
    let revenue = helpers::seed_account(&pool, &org.id, "4000", "Revenue", AccountType::Revenue).await;

    let entry = TransactionRepo::create(
        &pool,
        &org.id,
        &user.id,
        CreateJournalEntry {
            date: helpers::jan_15(),
            reference: Some("SPLIT-001".to_string()),
            description: "Split payment".to_string(),
            lines: vec![
                CreateJournalLine { account_id: cash.id.clone(), description: None, debit: 6_000, credit: 0 },
                CreateJournalLine { account_id: bank.id.clone(), description: None, debit: 4_000, credit: 0 },
                CreateJournalLine { account_id: revenue.id.clone(), description: None, debit: 0, credit: 10_000 },
            ],
        },
    )
    .await
    .unwrap();

    assert_eq!(entry.lines.len(), 3);
    let total_debits: i64 = entry.lines.iter().map(|l| l.debit).sum();
    let total_credits: i64 = entry.lines.iter().map(|l| l.credit).sum();
    assert_eq!(total_debits, total_credits);
}
