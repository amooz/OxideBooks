use oxidebooks_core::models::{CreateDunningRule, DunningRule, InvoiceReminder, OverdueInvoice};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct DunningRuleRow {
    id: Uuid,
    organization_id: Uuid,
    days_overdue: i32,
    reminder_level: i32,
    subject_template: String,
    body_template: String,
    is_active: bool,
    created_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct ReminderRow {
    id: Uuid,
    invoice_id: Uuid,
    rule_id: Option<Uuid>,
    to_address: String,
    level: i32,
    sent_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct OverdueRow {
    invoice_id: Uuid,
    invoice_number: String,
    contact_id: Uuid,
    days_overdue: i64,
    amount_due: i64,
    currency: String,
    last_reminder_level: Option<i32>,
}

fn rule_from_row(r: DunningRuleRow) -> DunningRule {
    DunningRule {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        days_overdue: r.days_overdue,
        reminder_level: r.reminder_level,
        subject_template: r.subject_template,
        body_template: r.body_template,
        is_active: r.is_active,
        created_at: r.created_at,
    }
}

fn reminder_from_row(r: ReminderRow) -> InvoiceReminder {
    InvoiceReminder {
        id: r.id.to_string(),
        invoice_id: r.invoice_id.to_string(),
        rule_id: r.rule_id.map(|u| u.to_string()),
        to_address: r.to_address,
        level: r.level,
        sent_at: r.sent_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

pub struct DunningRepo;

impl DunningRepo {
    pub async fn list_rules(pool: &PgPool, org_id: &str) -> Result<Vec<DunningRule>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<DunningRuleRow> = sqlx::query_as(
            "SELECT id, organization_id, days_overdue, reminder_level, \
             subject_template, body_template, is_active, created_at \
             FROM dunning_rules WHERE organization_id = $1 ORDER BY days_overdue",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(rule_from_row).collect())
    }

    pub async fn create_rule(
        pool: &PgPool,
        org_id: &str,
        input: CreateDunningRule,
    ) -> Result<DunningRule, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO dunning_rules \
             (organization_id, days_overdue, reminder_level, subject_template, body_template) \
             VALUES ($1,$2,$3,$4,$5) \
             ON CONFLICT (organization_id, days_overdue) DO UPDATE SET \
               reminder_level    = EXCLUDED.reminder_level, \
               subject_template  = EXCLUDED.subject_template, \
               body_template     = EXCLUDED.body_template \
             RETURNING id",
        )
        .bind(org_uuid)
        .bind(input.days_overdue)
        .bind(input.reminder_level)
        .bind(&input.subject_template)
        .bind(&input.body_template)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: DunningRuleRow = sqlx::query_as(
            "SELECT id, organization_id, days_overdue, reminder_level, \
             subject_template, body_template, is_active, created_at \
             FROM dunning_rules WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rule_from_row(row))
    }

    pub async fn delete_rule(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n = sqlx::query("DELETE FROM dunning_rules WHERE id = $1 AND organization_id = $2")
            .bind(id_uuid)
            .bind(org_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?
            .rows_affected();
        if n == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    pub async fn overdue_invoices(
        pool: &PgPool,
        org_id: &str,
    ) -> Result<Vec<OverdueInvoice>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<OverdueRow> = sqlx::query_as(
            "SELECT \
               i.id AS invoice_id, \
               i.invoice_number, \
               i.contact_id, \
               (CURRENT_DATE - i.due_date)::BIGINT AS days_overdue, \
               i.total_amount AS amount_due, \
               i.currency, \
               (SELECT MAX(r.level) FROM invoice_reminders r WHERE r.invoice_id = i.id) AS last_reminder_level \
             FROM invoices i \
             WHERE i.organization_id = $1 \
               AND i.status IN ('sent','overdue') \
               AND i.due_date < CURRENT_DATE \
             ORDER BY days_overdue DESC",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows
            .into_iter()
            .map(|r| OverdueInvoice {
                invoice_id: r.invoice_id.to_string(),
                invoice_number: r.invoice_number,
                contact_id: r.contact_id.to_string(),
                days_overdue: r.days_overdue,
                amount_due: r.amount_due,
                currency: r.currency,
                last_reminder_level: r.last_reminder_level,
            })
            .collect())
    }

    pub async fn record_reminder(
        pool: &PgPool,
        invoice_id: &str,
        rule_id: Option<&str>,
        to_address: &str,
        level: i32,
    ) -> Result<InvoiceReminder, DbError> {
        let invoice_uuid = parse_uuid(invoice_id)?;
        let rule_uuid = rule_id.map(parse_uuid).transpose()?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO invoice_reminders (invoice_id, rule_id, to_address, level) \
             VALUES ($1,$2,$3,$4) RETURNING id",
        )
        .bind(invoice_uuid)
        .bind(rule_uuid)
        .bind(to_address)
        .bind(level)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: ReminderRow = sqlx::query_as(
            "SELECT id, invoice_id, rule_id, to_address, level, sent_at \
             FROM invoice_reminders WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(reminder_from_row(row))
    }

    pub async fn list_reminders(
        pool: &PgPool,
        invoice_id: &str,
    ) -> Result<Vec<InvoiceReminder>, DbError> {
        let invoice_uuid = parse_uuid(invoice_id)?;
        let rows: Vec<ReminderRow> = sqlx::query_as(
            "SELECT id, invoice_id, rule_id, to_address, level, sent_at \
             FROM invoice_reminders WHERE invoice_id = $1 ORDER BY sent_at",
        )
        .bind(invoice_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(reminder_from_row).collect())
    }
}
