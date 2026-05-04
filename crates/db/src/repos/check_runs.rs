use oxidebooks_core::models::{CheckRun, CheckRunItem, CreateCheckRun};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct RunRow {
    id: Uuid,
    organization_id: Uuid,
    bank_account_id: Uuid,
    run_date: Date,
    status: String,
    starting_check_number: Option<i32>,
    notes: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn run_from_row(r: RunRow) -> CheckRun {
    CheckRun {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        bank_account_id: r.bank_account_id.to_string(),
        run_date: r.run_date,
        status: r.status,
        starting_check_number: r.starting_check_number,
        notes: r.notes,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

#[derive(sqlx::FromRow)]
struct ItemRow {
    id: Uuid,
    check_run_id: Uuid,
    payee_id: Option<Uuid>,
    payee_name: String,
    amount: i64,
    memo: Option<String>,
    check_number: Option<i32>,
    status: String,
    created_at: OffsetDateTime,
}

fn item_from_row(r: ItemRow) -> CheckRunItem {
    CheckRunItem {
        id: r.id.to_string(),
        check_run_id: r.check_run_id.to_string(),
        payee_id: r.payee_id.map(|u| u.to_string()),
        payee_name: r.payee_name,
        amount: r.amount,
        memo: r.memo,
        check_number: r.check_number,
        status: r.status,
        created_at: r.created_at,
    }
}

const RUN_COLS: &str = "id, organization_id, bank_account_id, run_date, status, \
                        starting_check_number, notes, created_at, updated_at";

const ITEM_COLS: &str =
    "id, check_run_id, payee_id, payee_name, amount, memo, check_number, status, created_at";

pub struct CheckRunRepo;

impl CheckRunRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<CheckRun>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<RunRow> = sqlx::query_as(&format!(
            "SELECT {RUN_COLS} FROM check_runs \
             WHERE organization_id = $1 ORDER BY run_date DESC, created_at DESC"
        ))
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(run_from_row).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<CheckRun, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: RunRow = sqlx::query_as(&format!(
            "SELECT {RUN_COLS} FROM check_runs WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(run_from_row(row))
    }

    pub async fn list_items(
        pool: &PgPool,
        org_id: &str,
        run_id: &str,
    ) -> Result<Vec<CheckRunItem>, DbError> {
        Self::get_by_id(pool, org_id, run_id).await?;
        let run_uuid = parse_uuid(run_id)?;
        let rows: Vec<ItemRow> = sqlx::query_as(&format!(
            "SELECT {ITEM_COLS} FROM check_run_items \
             WHERE check_run_id = $1 ORDER BY created_at ASC"
        ))
        .bind(run_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(item_from_row).collect())
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateCheckRun,
    ) -> Result<CheckRun, DbError> {
        if input.items.is_empty() {
            return Err(DbError::Conflict(
                "check run must have at least one item".into(),
            ));
        }
        for item in &input.items {
            if item.amount <= 0 {
                return Err(DbError::Conflict("item amount must be positive".into()));
            }
        }

        let org_uuid = parse_uuid(org_id)?;
        let bank_uuid = parse_uuid(&input.bank_account_id)?;

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        let run_id: Uuid = sqlx::query_scalar(
            "INSERT INTO check_runs \
             (organization_id, bank_account_id, run_date, starting_check_number, notes) \
             VALUES ($1,$2,$3,$4,$5) RETURNING id",
        )
        .bind(org_uuid)
        .bind(bank_uuid)
        .bind(input.run_date)
        .bind(input.starting_check_number)
        .bind(&input.notes)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        for item in input.items {
            let payee_uuid = item.payee_id.as_deref().map(parse_uuid).transpose()?;
            sqlx::query(
                "INSERT INTO check_run_items \
                 (check_run_id, payee_id, payee_name, amount, memo) \
                 VALUES ($1,$2,$3,$4,$5)",
            )
            .bind(run_id)
            .bind(payee_uuid)
            .bind(&item.payee_name)
            .bind(item.amount)
            .bind(&item.memo)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &run_id.to_string()).await
    }

    /// Assigns sequential check numbers and transitions all pending items + the run to printed.
    pub async fn print_run(pool: &PgPool, org_id: &str, id: &str) -> Result<CheckRun, DbError> {
        let run = Self::get_by_id(pool, org_id, id).await?;
        if run.status != "draft" {
            return Err(DbError::Conflict(
                "only draft check runs can be printed".into(),
            ));
        }

        let run_uuid = parse_uuid(id)?;
        let org_uuid = parse_uuid(org_id)?;
        let starting = run.starting_check_number.unwrap_or(1);

        // Fetch pending items in creation order to assign sequential numbers.
        let item_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM check_run_items \
             WHERE check_run_id = $1 AND status = 'pending' ORDER BY created_at ASC",
        )
        .bind(run_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        for (i, item_id) in item_ids.iter().enumerate() {
            let check_num = starting + i as i32;
            sqlx::query(
                "UPDATE check_run_items SET status = 'printed', check_number = $1 \
                 WHERE id = $2",
            )
            .bind(check_num)
            .bind(item_id)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        sqlx::query(
            "UPDATE check_runs SET status = 'printed', updated_at = NOW() \
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(run_uuid)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn void_run(pool: &PgPool, org_id: &str, id: &str) -> Result<CheckRun, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows = sqlx::query(
            "UPDATE check_runs SET status = 'voided', updated_at = NOW() \
             WHERE organization_id = $1 AND id = $2 AND status != 'voided'",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if rows == 0 {
            return Err(DbError::NotFound);
        }
        sqlx::query(
            "UPDATE check_run_items SET status = 'voided' \
             WHERE check_run_id = $1 AND status != 'voided'",
        )
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn void_item(
        pool: &PgPool,
        org_id: &str,
        run_id: &str,
        item_id: &str,
    ) -> Result<CheckRunItem, DbError> {
        Self::get_by_id(pool, org_id, run_id).await?;
        let run_uuid = parse_uuid(run_id)?;
        let item_uuid = parse_uuid(item_id)?;
        let rows = sqlx::query(
            "UPDATE check_run_items SET status = 'voided' \
             WHERE check_run_id = $1 AND id = $2 AND status != 'voided'",
        )
        .bind(run_uuid)
        .bind(item_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if rows == 0 {
            return Err(DbError::NotFound);
        }
        let row: ItemRow = sqlx::query_as(&format!(
            "SELECT {ITEM_COLS} FROM check_run_items WHERE id = $1"
        ))
        .bind(item_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(item_from_row(row))
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
