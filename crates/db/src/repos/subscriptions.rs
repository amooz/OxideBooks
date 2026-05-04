use oxidebooks_core::models::{
    CreateSubscription, CreateSubscriptionPlan, Subscription, SubscriptionPlan, UpdateSubscription,
    UpdateSubscriptionPlan,
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
    name: String,
    description: Option<String>,
    price: i64,
    currency: String,
    billing_cycle: String,
    trial_days: i32,
    is_active: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn plan_from_row(r: PlanRow) -> SubscriptionPlan {
    SubscriptionPlan {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        name: r.name,
        description: r.description,
        price: r.price,
        currency: r.currency,
        billing_cycle: r.billing_cycle,
        trial_days: r.trial_days,
        is_active: r.is_active,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

#[derive(sqlx::FromRow)]
struct SubRow {
    id: Uuid,
    organization_id: Uuid,
    plan_id: Uuid,
    plan_name: String,
    plan_price: i64,
    contact_id: Uuid,
    status: String,
    quantity: i32,
    current_period_start: Date,
    current_period_end: Date,
    trial_end: Option<Date>,
    cancel_at_period_end: bool,
    cancelled_at: Option<OffsetDateTime>,
    notes: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn sub_from_row(r: SubRow) -> Subscription {
    let billing_amount = r.plan_price * r.quantity as i64;
    Subscription {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        plan_id: r.plan_id.to_string(),
        plan_name: r.plan_name,
        contact_id: r.contact_id.to_string(),
        status: r.status,
        quantity: r.quantity,
        unit_price: r.plan_price,
        billing_amount,
        current_period_start: r.current_period_start,
        current_period_end: r.current_period_end,
        trial_end: r.trial_end,
        cancel_at_period_end: r.cancel_at_period_end,
        cancelled_at: r.cancelled_at,
        notes: r.notes,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

const PLAN_COLS: &str = "id, organization_id, name, description, price, currency,
     billing_cycle, trial_days, is_active, created_at, updated_at";

const SUB_COLS: &str =
    "s.id, s.organization_id, s.plan_id, p.name AS plan_name, p.price AS plan_price,
     s.contact_id, s.status, s.quantity, s.current_period_start, s.current_period_end,
     s.trial_end, s.cancel_at_period_end, s.cancelled_at, s.notes, s.created_at, s.updated_at";

fn next_period_end(start: Date, cycle: &str) -> Date {
    use time::Month;
    match cycle {
        "weekly" => start + time::Duration::weeks(1),
        "quarterly" => {
            let (y, m, d) = (start.year(), start.month() as u8, start.day());
            let new_m = m + 3;
            let (ny, nm) = if new_m > 12 {
                (y + 1, Month::try_from(new_m - 12).unwrap_or(Month::January))
            } else {
                (y, Month::try_from(new_m).unwrap_or(Month::December))
            };
            Date::from_calendar_date(ny, nm, d.min(28)).unwrap_or(start)
        }
        "annually" => {
            Date::from_calendar_date(start.year() + 1, start.month(), start.day().min(28))
                .unwrap_or(start)
        }
        _ => {
            // monthly
            let (y, m, d) = (start.year(), start.month() as u8, start.day());
            let (ny, nm) = if m == 12 {
                (y + 1, Month::January)
            } else {
                (y, Month::try_from(m + 1).unwrap_or(Month::December))
            };
            Date::from_calendar_date(ny, nm, d.min(28)).unwrap_or(start)
        }
    }
}

pub struct SubscriptionRepo;

impl SubscriptionRepo {
    // ── Plans ─────────────────────────────────────────────────────────────────

    pub async fn list_plans(pool: &PgPool, org_id: &str) -> Result<Vec<SubscriptionPlan>, DbError> {
        let org = parse_uuid(org_id)?;
        let rows: Vec<PlanRow> = sqlx::query_as(&format!(
            "SELECT {PLAN_COLS} FROM subscription_plans WHERE organization_id = $1 ORDER BY name"
        ))
        .bind(org)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(plan_from_row).collect())
    }

    pub async fn get_plan(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<SubscriptionPlan, DbError> {
        let org = parse_uuid(org_id)?;
        let pid = parse_uuid(id)?;
        let row: PlanRow = sqlx::query_as(&format!(
            "SELECT {PLAN_COLS} FROM subscription_plans WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org)
        .bind(pid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(plan_from_row(row))
    }

    pub async fn create_plan(
        pool: &PgPool,
        org_id: &str,
        input: CreateSubscriptionPlan,
    ) -> Result<SubscriptionPlan, DbError> {
        let org = parse_uuid(org_id)?;
        let valid_cycles = ["weekly", "monthly", "quarterly", "annually"];
        if !valid_cycles.contains(&input.billing_cycle.as_str()) {
            return Err(DbError::Conflict(format!(
                "invalid billing_cycle '{}'",
                input.billing_cycle
            )));
        }
        let row: PlanRow = sqlx::query_as(&format!(
            "INSERT INTO subscription_plans
                (organization_id, name, description, price, currency, billing_cycle, trial_days)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING {PLAN_COLS}"
        ))
        .bind(org)
        .bind(&input.name)
        .bind(&input.description)
        .bind(input.price)
        .bind(&input.currency)
        .bind(&input.billing_cycle)
        .bind(input.trial_days)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(plan_from_row(row))
    }

    pub async fn update_plan(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateSubscriptionPlan,
    ) -> Result<SubscriptionPlan, DbError> {
        let org = parse_uuid(org_id)?;
        let pid = parse_uuid(id)?;
        let row: Option<PlanRow> = sqlx::query_as(&format!(
            "UPDATE subscription_plans
             SET name        = COALESCE($3, name),
                 description = COALESCE($4, description),
                 price       = COALESCE($5, price),
                 is_active   = COALESCE($6, is_active),
                 updated_at  = now()
             WHERE organization_id = $1 AND id = $2
             RETURNING {PLAN_COLS}"
        ))
        .bind(org)
        .bind(pid)
        .bind(&input.name)
        .bind(&input.description)
        .bind(input.price)
        .bind(input.is_active)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;
        row.map(plan_from_row).ok_or(DbError::NotFound)
    }

    // ── Subscriptions ─────────────────────────────────────────────────────────

    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        status: Option<&str>,
        contact_id: Option<&str>,
    ) -> Result<Vec<Subscription>, DbError> {
        let org = parse_uuid(org_id)?;
        let contact = contact_id.map(parse_uuid).transpose()?;
        let rows: Vec<SubRow> = sqlx::query_as(&format!(
            "SELECT {SUB_COLS}
             FROM subscriptions s
             JOIN subscription_plans p ON p.id = s.plan_id
             WHERE s.organization_id = $1
               AND ($2::TEXT IS NULL OR s.status = $2)
               AND ($3::UUID IS NULL OR s.contact_id = $3)
             ORDER BY s.created_at DESC"
        ))
        .bind(org)
        .bind(status)
        .bind(contact)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(sub_from_row).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<Subscription, DbError> {
        let org = parse_uuid(org_id)?;
        let sid = parse_uuid(id)?;
        let row: SubRow = sqlx::query_as(&format!(
            "SELECT {SUB_COLS}
             FROM subscriptions s
             JOIN subscription_plans p ON p.id = s.plan_id
             WHERE s.organization_id = $1 AND s.id = $2"
        ))
        .bind(org)
        .bind(sid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(sub_from_row(row))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateSubscription,
    ) -> Result<Subscription, DbError> {
        let org = parse_uuid(org_id)?;
        let plan_id = parse_uuid(&input.plan_id)?;
        let contact_id = parse_uuid(&input.contact_id)?;

        // Fetch plan to determine billing cycle and trial
        let plan: Option<(String, i32)> = sqlx::query_as(
            "SELECT billing_cycle, trial_days FROM subscription_plans
             WHERE organization_id = $1 AND id = $2 AND is_active = TRUE",
        )
        .bind(org)
        .bind(plan_id)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        let (billing_cycle, trial_days) = plan.ok_or(DbError::NotFound)?;

        let today = time::OffsetDateTime::now_utc().date();
        let trial_end = if trial_days > 0 {
            Some(today + time::Duration::days(trial_days as i64))
        } else {
            None
        };

        let period_start = trial_end.unwrap_or(today);
        let period_end = next_period_end(period_start, &billing_cycle);
        let status = if trial_days > 0 { "trialing" } else { "active" };

        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO subscriptions
                (id, organization_id, plan_id, contact_id, status, quantity,
                 current_period_start, current_period_end, trial_end, notes)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(id)
        .bind(org)
        .bind(plan_id)
        .bind(contact_id)
        .bind(status)
        .bind(input.quantity)
        .bind(period_start)
        .bind(period_end)
        .bind(trial_end)
        .bind(&input.notes)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateSubscription,
    ) -> Result<Subscription, DbError> {
        let org = parse_uuid(org_id)?;
        let sid = parse_uuid(id)?;
        let n = sqlx::query(
            "UPDATE subscriptions
             SET quantity             = COALESCE($3, quantity),
                 cancel_at_period_end = COALESCE($4, cancel_at_period_end),
                 notes                = COALESCE($5, notes),
                 updated_at           = now()
             WHERE organization_id = $1 AND id = $2
               AND status IN ('trialing','active','past_due')",
        )
        .bind(org)
        .bind(sid)
        .bind(input.quantity)
        .bind(input.cancel_at_period_end)
        .bind(input.notes)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::NotFound);
        }
        Self::get_by_id(pool, org_id, id).await
    }

    /// Immediately cancel a subscription (sets status = cancelled).
    pub async fn cancel(pool: &PgPool, org_id: &str, id: &str) -> Result<Subscription, DbError> {
        let org = parse_uuid(org_id)?;
        let sid = parse_uuid(id)?;
        let n = sqlx::query(
            "UPDATE subscriptions
             SET status = 'cancelled', cancelled_at = now(), updated_at = now()
             WHERE organization_id = $1 AND id = $2
               AND status IN ('trialing','active','past_due')",
        )
        .bind(org)
        .bind(sid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::Conflict(
                "subscription cannot be cancelled in its current state".into(),
            ));
        }
        Self::get_by_id(pool, org_id, id).await
    }

    /// Advance the subscription's billing period (called when payment succeeds).
    pub async fn renew(pool: &PgPool, org_id: &str, id: &str) -> Result<Subscription, DbError> {
        let org = parse_uuid(org_id)?;
        let sid = parse_uuid(id)?;

        let sub: Option<(Date, String, bool)> = sqlx::query_as(
            "SELECT s.current_period_end, p.billing_cycle, s.cancel_at_period_end
             FROM subscriptions s JOIN subscription_plans p ON p.id = s.plan_id
             WHERE s.organization_id = $1 AND s.id = $2 AND s.status IN ('active','trialing')",
        )
        .bind(org)
        .bind(sid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        let (current_end, billing_cycle, cancel_at_end) = sub.ok_or(DbError::NotFound)?;

        if cancel_at_end {
            sqlx::query(
                "UPDATE subscriptions
                 SET status = 'cancelled', cancelled_at = now(), updated_at = now()
                 WHERE id = $1",
            )
            .bind(sid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;
        } else {
            let new_end = next_period_end(current_end, &billing_cycle);
            sqlx::query(
                "UPDATE subscriptions
                 SET current_period_start = $2, current_period_end = $3,
                     status = 'active', updated_at = now()
                 WHERE id = $1",
            )
            .bind(sid)
            .bind(current_end)
            .bind(new_end)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;
        }

        Self::get_by_id(pool, org_id, id).await
    }

    /// Create a draft invoice for the subscription's current billing period,
    /// then advance the period (same as `renew`).
    pub async fn bill(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<oxidebooks_core::models::Invoice, DbError> {
        use oxidebooks_core::models::{CreateInvoice, CreateInvoiceLine, InvoiceType};

        let sub = Self::get_by_id(pool, org_id, id).await?;
        if !matches!(sub.status.as_str(), "active" | "trialing") {
            return Err(DbError::Conflict(
                "subscription must be active or trialing to bill".into(),
            ));
        }

        let description = format!(
            "{} \u{2014} {} to {}",
            sub.plan_name, sub.current_period_start, sub.current_period_end,
        );

        let input = CreateInvoice {
            contact_id: sub.contact_id.clone(),
            invoice_type: InvoiceType::Invoice,
            date: time::OffsetDateTime::now_utc().date(),
            due_date: sub.current_period_end,
            currency: None,
            exchange_rate: None,
            notes: None,
            global_discount_pct: 0,
            lines: vec![CreateInvoiceLine {
                description,
                account_id: None,
                product_id: None,
                quantity: sub.quantity as i64 * 100,
                unit_price: sub.unit_price,
                tax_rate: None,
                discount_pct: 0,
                variant_id: None,
            }],
        };

        let invoice = super::invoices::InvoiceRepo::create(pool, org_id, input).await?;
        Self::renew(pool, org_id, id).await?;
        Ok(invoice)
    }

    /// Bill all active subscriptions whose current_period_end is on or before `as_of`.
    /// Returns the list of generated invoice IDs and a count of failures.
    pub async fn bill_due(
        pool: &PgPool,
        org_id: &str,
        as_of: Date,
    ) -> Result<BillingRunResult, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM subscriptions
             WHERE organization_id = $1
               AND status IN ('active','trialing')
               AND current_period_end <= $2",
        )
        .bind(org_uuid)
        .bind(as_of)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut invoiced: Vec<String> = Vec::new();
        let mut failed: Vec<String> = Vec::new();

        for sub_id in ids {
            let sid = sub_id.to_string();
            match Self::bill(pool, org_id, &sid).await {
                Ok(inv) => invoiced.push(inv.id),
                Err(_) => failed.push(sid),
            }
        }

        Ok(BillingRunResult {
            invoiced_count: invoiced.len() as i64,
            failed_count: failed.len() as i64,
            invoice_ids: invoiced,
            failed_subscription_ids: failed,
        })
    }
}

#[derive(Debug, serde::Serialize)]
pub struct BillingRunResult {
    pub invoiced_count: i64,
    pub failed_count: i64,
    pub invoice_ids: Vec<String>,
    pub failed_subscription_ids: Vec<String>,
}
