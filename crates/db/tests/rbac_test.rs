mod helpers;

use oxidebooks_db::repos::{
    PermissionRepo, RoleRepo, ROLE_ACCOUNTANT_ID, ROLE_ADMIN_ID, ROLE_OWNER_ID, ROLE_VIEWER_ID,
};

// ── PermissionRepo ────────────────────────────────────────────────────────────

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn list_permissions_returns_all_system_permissions(pool: sqlx::PgPool) {
    let perms = PermissionRepo::list(&pool).await.unwrap();
    // 15 permissions seeded in the migration
    assert_eq!(perms.len(), 15);
    let names: Vec<&str> = perms.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"accounts:read"));
    assert!(names.contains(&"accounts:write"));
    assert!(names.contains(&"accounts:delete"));
    assert!(names.contains(&"roles:write"));
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn list_for_role_returns_viewer_permissions(pool: sqlx::PgPool) {
    let perms = PermissionRepo::list_for_role(&pool, ROLE_VIEWER_ID)
        .await
        .unwrap();
    let names: Vec<&str> = perms.iter().map(|p| p.as_str()).collect();
    assert!(names.contains(&"accounts:read"));
    assert!(names.contains(&"transactions:read"));
    assert!(names.contains(&"contacts:read"));
    assert!(names.contains(&"invoices:read"));
    assert!(names.contains(&"reports:read"));
    // viewer must NOT have write or admin permissions
    assert!(!names.contains(&"accounts:write"));
    assert!(!names.contains(&"roles:write"));
    assert!(!names.contains(&"users:delete"));
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn list_for_role_returns_accountant_permissions(pool: sqlx::PgPool) {
    let perms = PermissionRepo::list_for_role(&pool, ROLE_ACCOUNTANT_ID)
        .await
        .unwrap();
    let names: Vec<&str> = perms.iter().map(|p| p.as_str()).collect();
    assert!(names.contains(&"accounts:write"));
    assert!(names.contains(&"transactions:write"));
    // accountant must NOT have delete or user management
    assert!(!names.contains(&"accounts:delete"));
    assert!(!names.contains(&"users:write"));
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn list_for_role_returns_admin_permissions(pool: sqlx::PgPool) {
    let perms = PermissionRepo::list_for_role(&pool, ROLE_ADMIN_ID)
        .await
        .unwrap();
    let names: Vec<&str> = perms.iter().map(|p| p.as_str()).collect();
    assert!(names.contains(&"accounts:delete"));
    assert!(names.contains(&"users:write"));
    assert!(names.contains(&"roles:read"));
    // admin must NOT have roles:write or users:delete
    assert!(!names.contains(&"roles:write"));
    assert!(!names.contains(&"users:delete"));
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn list_for_role_returns_owner_all_permissions(pool: sqlx::PgPool) {
    let all = PermissionRepo::list(&pool).await.unwrap();
    let owner = PermissionRepo::list_for_role(&pool, ROLE_OWNER_ID)
        .await
        .unwrap();
    assert_eq!(owner.len(), all.len(), "owner must have every permission");
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn list_for_user_returns_permissions_via_role(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    let user = helpers::seed_user(&pool, &org.id).await; // owner role
    let perms = PermissionRepo::list_for_user(&pool, &user.id)
        .await
        .unwrap();
    // owner should have all 15 permissions
    assert_eq!(perms.len(), 15);
}

// ── RoleRepo ──────────────────────────────────────────────────────────────────

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn list_roles_includes_system_roles(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    let roles = RoleRepo::list(&pool, &org.id).await.unwrap();
    let names: Vec<&str> = roles.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"owner"));
    assert!(names.contains(&"admin"));
    assert!(names.contains(&"accountant"));
    assert!(names.contains(&"viewer"));
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn list_roles_includes_permissions_inline(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    let roles = RoleRepo::list(&pool, &org.id).await.unwrap();
    let viewer = roles.iter().find(|r| r.name == "viewer").unwrap();
    assert!(viewer.permissions.contains(&"accounts:read".to_string()));
    assert!(!viewer.permissions.contains(&"accounts:write".to_string()));
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn create_custom_role_is_scoped_to_org(pool: sqlx::PgPool) {
    let org1 = helpers::seed_org(&pool).await;
    let org2 = oxidebooks_db::repos::OrganizationRepo::create(
        &pool,
        oxidebooks_core::models::CreateOrganization {
            name: "Other".to_string(),
            currency: "USD".to_string(),
            fiscal_year_start: None,
        },
    )
    .await
    .unwrap();

    RoleRepo::create(&pool, &org1.id, "billing-manager")
        .await
        .unwrap();

    let roles1 = RoleRepo::list(&pool, &org1.id).await.unwrap();
    let roles2 = RoleRepo::list(&pool, &org2.id).await.unwrap();

    assert!(roles1.iter().any(|r| r.name == "billing-manager"));
    assert!(!roles2.iter().any(|r| r.name == "billing-manager"));
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn create_custom_role_starts_with_no_permissions(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    let role = RoleRepo::create(&pool, &org.id, "analyst").await.unwrap();
    assert!(role.permissions.is_empty());
    assert!(!role.is_system);
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn assign_permission_to_custom_role(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    let role = RoleRepo::create(&pool, &org.id, "reporter").await.unwrap();

    RoleRepo::assign_permission(&pool, &org.id, &role.id, "reports:read")
        .await
        .unwrap();

    let perms = PermissionRepo::list_for_role(&pool, &role.id)
        .await
        .unwrap();
    assert!(perms.contains(&"reports:read".to_string()));
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn assign_permission_is_idempotent(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    let role = RoleRepo::create(&pool, &org.id, "reporter").await.unwrap();

    RoleRepo::assign_permission(&pool, &org.id, &role.id, "reports:read")
        .await
        .unwrap();
    RoleRepo::assign_permission(&pool, &org.id, &role.id, "reports:read")
        .await
        .unwrap();

    let perms = PermissionRepo::list_for_role(&pool, &role.id)
        .await
        .unwrap();
    assert_eq!(
        perms
            .iter()
            .filter(|p| p.as_str() == "reports:read")
            .count(),
        1
    );
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn remove_permission_from_custom_role(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    let role = RoleRepo::create(&pool, &org.id, "limited").await.unwrap();

    RoleRepo::assign_permission(&pool, &org.id, &role.id, "accounts:read")
        .await
        .unwrap();
    RoleRepo::assign_permission(&pool, &org.id, &role.id, "invoices:read")
        .await
        .unwrap();

    RoleRepo::remove_permission(&pool, &org.id, &role.id, "invoices:read")
        .await
        .unwrap();

    let perms = PermissionRepo::list_for_role(&pool, &role.id)
        .await
        .unwrap();
    assert!(perms.contains(&"accounts:read".to_string()));
    assert!(!perms.contains(&"invoices:read".to_string()));
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn get_by_id_returns_role(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    let created = RoleRepo::create(&pool, &org.id, "custom").await.unwrap();
    let fetched = RoleRepo::get_by_id(&pool, &org.id, &created.id)
        .await
        .unwrap();
    assert_eq!(fetched.name, "custom");
    assert_eq!(fetched.id, created.id);
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn get_by_id_not_found_for_other_org(pool: sqlx::PgPool) {
    let org1 = helpers::seed_org(&pool).await;
    let org2 = oxidebooks_db::repos::OrganizationRepo::create(
        &pool,
        oxidebooks_core::models::CreateOrganization {
            name: "Other".to_string(),
            currency: "USD".to_string(),
            fiscal_year_start: None,
        },
    )
    .await
    .unwrap();

    let role = RoleRepo::create(&pool, &org1.id, "private-role")
        .await
        .unwrap();
    let result = RoleRepo::get_by_id(&pool, &org2.id, &role.id).await;
    assert!(result.is_err());
}

#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]
async fn duplicate_role_name_in_same_org_is_conflict(pool: sqlx::PgPool) {
    let org = helpers::seed_org(&pool).await;
    RoleRepo::create(&pool, &org.id, "analyst").await.unwrap();
    let result = RoleRepo::create(&pool, &org.id, "analyst").await;
    assert!(result.is_err());
}
