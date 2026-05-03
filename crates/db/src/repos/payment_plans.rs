use oxidebooks_core::models::{
    CreatePaymentPlan, PayInstallment, PaymentPlan, PaymentPlanInstallment,
};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct PlanRow {
    id: Uuid,
    organization_id: Uuid,
    invoice_id: Uuid,
    contact_id: Uuid,
    description: Option<String>,
    total_amount: i64,
    paid_amount: i64,
    status: String,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct InstallmentRow {
    id: Uuid,
    plan_id: Uuid,
    due_date: Date,
    amount: i64,
    paid_amount: i64,
    status: String,
    sort_order: i32,
}

fn installment_from_row(r: InstallmentRow) -> PaymentPlanInstallment {
    PaymentPlanInstallment {
        id: r.id.to_string(),
        plan_id: r.plan_id.to_string(),
        due_date: r.due_date,
        amount: r.amount,
        paid_amount: r.paid_amount,
        status: r.status,
        sort_order: r.sort_order,
    }
}

async fn fetch_installments(
    pool: &PgPool,
    plan_id: Uuid,
) -> Result<Vec<PaymentPlanInstallment>, DbError> {
    let rows: Vec<InstallmentRow> = sqlx::query_as(
        "SELECT id, plan_id, due_date, amount, paid_amount, status, sort_order
         FROM payment_plan_installments WHERE plan_id = $1 ORDER BY sort_order, due_date",
    )
    .bind(plan_id)
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_err)?;
    Ok(rows.into_iter().map(installment_from_row).collect())
}

async fn plan_from_row(pool: &PgPool, r: PlanRow) -> Result<PaymentPlan, DbError> {
    let installments = fetch_installments(pool, r.id).await?;
    let remaining = r.total_amount - r.paid_amount;
    Ok(PaymentPlan {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        invoice_id: r.invoice_id.to_string(),
        contact_id: r.contact_id.to_string(),
        description: r.description,
        total_amount: r.total_amount,
        paid_amount: r.paid_amount,
        remaining_amount: remaining,
        status: r.status,
        installments,
        created_at: r.created_at,
        updated_at: r.updated_at,
    })
}

const COLS: &str = "id, organization_id, invoice_id, contact_id, description,
     total_amount, paid_amount, status, created_at, updated_at";

pub struct PaymentPlanRepo;

impl PaymentPlanRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        invoice_id: Option<&str>,
    ) -> Result<Vec<PaymentPlan>, DbError> {
        let org = parse_uuid(org_id)?;
        let inv = invoice_id.map(parse_uuid).transpose()?;
        let rows: Vec<PlanRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM payment_plans
             WHERE organization_id = $1
               AND ($2::UUID IS NULL OR invoice_id = $2)
             ORDER BY created_at DESC"
        ))
        .bind(org)
        .bind(inv)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(plan_from_row(pool, r).await?);
        }
        Ok(out)
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<PaymentPlan, DbError> {
        let org = parse_uuid(org_id)?;
        let pid = parse_uuid(id)?;
        let row: PlanRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM payment_plans WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org)
        .bind(pid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        plan_from_row(pool, row).await
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreatePaymentPlan,
    ) -> Result<PaymentPlan, DbError> {
        let org = parse_uuid(org_id)?;
        let inv_id = parse_uuid(&input.invoice_id)?;

        if input.installments.is_empty() {
            return Err(DbError::Conflict(
                "payment plan must have at least one installment".into(),
            ));
        }

        // Fetch contact_id from invoice
        let contact_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT contact_id FROM invoices WHERE organization_id = $1 AND id = $2",
        )
        .bind(org)
        .bind(inv_id)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;
        let contact_id = contact_id.ok_or(DbError::NotFound)?;

        let total_amount: i64 = input.installments.iter().map(|i| i.amount).sum();

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;
        let id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO payment_plans
                (id, organization_id, invoice_id, contact_id, description, total_amount)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id)
        .bind(org)
        .bind(inv_id)
        .bind(contact_id)
        .bind(&input.description)
        .bind(total_amount)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        for (i, inst) in input.installments.iter().enumerate() {
            sqlx::query(
                "INSERT INTO payment_plan_installments
                    (plan_id, due_date, amount, sort_order)
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(id)
            .bind(inst.due_date)
            .bind(inst.amount)
            .bind(i as i32)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn pay_installment(
        pool: &PgPool,
        org_id: &str,
        plan_id: &str,
        installment_id: &str,
        input: PayInstallment,
    ) -> Result<PaymentPlan, DbError> {
        let org = parse_uuid(org_id)?;
        let pid = parse_uuid(plan_id)?;
        let iid = parse_uuid(installment_id)?;

        if input.amount <= 0 {
            return Err(DbError::Conflict("amount must be positive".into()));
        }

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        // Fetch installment
        let inst: Option<(i64, i64)> = sqlx::query_as(
            "SELECT amount, paid_amount FROM payment_plan_installments
             WHERE id = $1 AND plan_id = $2 FOR UPDATE",
        )
        .bind(iid)
        .bind(pid)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        let (inst_amount, already_paid) = inst.ok_or(DbError::NotFound)?;
        let remaining = inst_amount - already_paid;
        if input.amount > remaining {
            return Err(DbError::Conflict(format!(
                "amount exceeds remaining balance of {remaining}"
            )));
        }

        let new_paid = already_paid + input.amount;
        let new_status = if new_paid >= inst_amount {
            "paid"
        } else {
            "partial"
        };

        sqlx::query(
            "UPDATE payment_plan_installments
             SET paid_amount = $2, status = $3
             WHERE id = $1",
        )
        .bind(iid)
        .bind(new_paid)
        .bind(new_status)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        // Update plan paid_amount and status
        let plan_paid: i64 = sqlx::query_scalar(
            "UPDATE payment_plans
             SET paid_amount = paid_amount + $2, updated_at = now()
             WHERE organization_id = $3 AND id = $1
             RETURNING paid_amount",
        )
        .bind(pid)
        .bind(input.amount)
        .bind(org)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        let plan_total: i64 =
            sqlx::query_scalar("SELECT total_amount FROM payment_plans WHERE id = $1")
                .bind(pid)
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_err)?;

        if plan_paid >= plan_total {
            sqlx::query(
                "UPDATE payment_plans SET status = 'completed', updated_at = now()
                 WHERE id = $1",
            )
            .bind(pid)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, plan_id).await
    }

    pub async fn cancel(pool: &PgPool, org_id: &str, id: &str) -> Result<PaymentPlan, DbError> {
        let org = parse_uuid(org_id)?;
        let pid = parse_uuid(id)?;
        let n = sqlx::query(
            "UPDATE payment_plans SET status = 'cancelled', updated_at = now()
             WHERE organization_id = $1 AND id = $2 AND status = 'active'",
        )
        .bind(org)
        .bind(pid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::Conflict(
                "payment plan must be active to cancel".into(),
            ));
        }
        Self::get_by_id(pool, org_id, id).await
    }
}
