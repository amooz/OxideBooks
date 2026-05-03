use oxidebooks_core::models::{CreateInvoice, Frequency};
use oxidebooks_db::repos::{InvoiceRepo, RecurringRepo};
use sqlx::PgPool;
use time::{Date, Duration, Month};
use tracing::{error, info};

pub fn spawn(pool: PgPool) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
            if let Err(e) = run_due_schedules(&pool).await {
                error!(error = %e, "recurring scheduler error");
            }
        }
    });
}

async fn run_due_schedules(pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let today = time::OffsetDateTime::now_utc().date();
    let schedules = RecurringRepo::list_due(pool, today).await?;

    for schedule in schedules {
        // Deserialize template to CreateInvoice
        let template: CreateInvoice = match serde_json::from_value(schedule.template.clone()) {
            Ok(t) => t,
            Err(e) => {
                error!(
                    schedule_id = %schedule.id,
                    error = %e,
                    "failed to deserialize recurring template"
                );
                continue;
            }
        };

        // Create the invoice (org is embedded in the template contact/data)
        match InvoiceRepo::create(pool, &schedule.organization_id, template).await {
            Ok(inv) => {
                info!(
                    schedule_id = %schedule.id,
                    invoice_id  = %inv.id,
                    "recurring invoice generated"
                );
            }
            Err(e) => {
                error!(schedule_id = %schedule.id, error = %e, "failed to create recurring invoice");
                continue;
            }
        }

        let new_due = advance_date(
            schedule.next_due_date,
            schedule.frequency,
            schedule.interval_count,
        );
        if let Err(e) = RecurringRepo::advance(pool, &schedule.id, new_due).await {
            error!(schedule_id = %schedule.id, error = %e, "failed to advance recurring schedule");
        }
    }

    Ok(())
}

fn advance_date(from: Date, freq: Frequency, n: i32) -> Date {
    match freq {
        Frequency::Weekly => from + Duration::weeks(n as i64),
        Frequency::Monthly => add_months(from, n),
        Frequency::Quarterly => add_months(from, n * 3),
        Frequency::Yearly => add_months(from, n * 12),
    }
}

fn add_months(date: Date, months: i32) -> Date {
    let total_months = date.month() as i32 - 1 + months;
    let year = date.year() + total_months / 12;
    let month_num = (total_months % 12 + 1) as u8;
    let month = Month::try_from(month_num).unwrap_or(Month::January);
    let day = date.day().min(days_in_month(year, month));
    Date::from_calendar_date(year, month, day).unwrap_or(date)
}

fn days_in_month(year: i32, month: Month) -> u8 {
    match month {
        Month::January
        | Month::March
        | Month::May
        | Month::July
        | Month::August
        | Month::October
        | Month::December => 31,
        Month::April | Month::June | Month::September | Month::November => 30,
        Month::February => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
    }
}
