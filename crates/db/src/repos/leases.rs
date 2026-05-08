use oxidebooks_core::models::{
    CreateLease, Lease, LeasePayment, LeaseScheduleLine, RecordLeasePayment, TerminateLease,
};
use sqlx::PgPool;
use time::{Date, Month, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

fn parse_date(s: &str) -> Result<Date, DbError> {
    Date::parse(s, &time::format_description::well_known::Iso8601::DEFAULT)
        .map_err(|_| DbError::Conflict(format!("invalid date: {s}")))
}

fn add_months(date: Date, months: i32) -> Date {
    let total = date.month() as i32 - 1 + months;
    let year = date.year() + total.div_euclid(12);
    let month = Month::try_from((total.rem_euclid(12) + 1) as u8).expect("valid month");
    Date::from_calendar_date(year, month, date.day().min(days_in_month(year, month)))
        .expect("valid date")
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
            if year % 400 == 0 || (year % 4 == 0 && year % 100 != 0) {
                29
            } else {
                28
            }
        }
    }
}

fn months_between(from: Date, to: Date) -> i32 {
    (to.year() - from.year()) * 12 + to.month() as i32 - from.month() as i32
}

/// Compute the present value of an ordinary annuity.
/// payment: payment per period; rate: rate per period (0.0 = zero-rate)
/// n: number of periods
fn annuity_pv(payment: f64, rate: f64, n: i32) -> f64 {
    if rate == 0.0 || n == 0 {
        return payment * n as f64;
    }
    payment * (1.0 - (1.0 + rate).powi(-n)) / rate
}

#[derive(sqlx::FromRow)]
struct LeaseRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    description: Option<String>,
    lease_type: String,
    asset_account_id: Option<Uuid>,
    liability_account_id: Option<Uuid>,
    expense_account_id: Option<Uuid>,
    commencement_date: Date,
    end_date: Date,
    payment_amount: i64,
    payment_frequency: String,
    discount_rate_bps: i32,
    initial_rou_asset: i64,
    initial_liability: i64,
    status: String,
    terminated_at: Option<Date>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn lease_from_row(r: LeaseRow) -> Lease {
    Lease {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        name: r.name,
        description: r.description,
        lease_type: r.lease_type,
        asset_account_id: r.asset_account_id.map(|u| u.to_string()),
        liability_account_id: r.liability_account_id.map(|u| u.to_string()),
        expense_account_id: r.expense_account_id.map(|u| u.to_string()),
        commencement_date: r.commencement_date,
        end_date: r.end_date,
        payment_amount: r.payment_amount,
        payment_frequency: r.payment_frequency,
        discount_rate_bps: r.discount_rate_bps,
        initial_rou_asset: r.initial_rou_asset,
        initial_liability: r.initial_liability,
        status: r.status,
        terminated_at: r.terminated_at,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

const COLS: &str =
    "id, organization_id, name, description, lease_type, asset_account_id, liability_account_id, \
     expense_account_id, commencement_date, end_date, payment_amount, payment_frequency, \
     discount_rate_bps, initial_rou_asset, initial_liability, status, terminated_at, \
     created_at, updated_at";

pub struct LeaseRepo;

impl LeaseRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        status: Option<&str>,
    ) -> Result<Vec<Lease>, DbError> {
        let org = parse_uuid(org_id)?;
        let rows: Vec<LeaseRow> = if let Some(s) = status {
            sqlx::query_as(&format!(
                "SELECT {COLS} FROM leases WHERE organization_id = $1 AND status = $2 \
                 ORDER BY commencement_date"
            ))
            .bind(org)
            .bind(s)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(&format!(
                "SELECT {COLS} FROM leases WHERE organization_id = $1 \
                 ORDER BY commencement_date"
            ))
            .bind(org)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        };
        Ok(rows.into_iter().map(lease_from_row).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<Lease, DbError> {
        let org = parse_uuid(org_id)?;
        let lid = parse_uuid(id)?;
        let row: Option<LeaseRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM leases WHERE id = $1 AND organization_id = $2"
        ))
        .bind(lid)
        .bind(org)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;
        row.map(lease_from_row).ok_or(DbError::NotFound)
    }

    pub async fn create(pool: &PgPool, org_id: &str, input: CreateLease) -> Result<Lease, DbError> {
        let valid_types = ["finance", "operating"];
        if !valid_types.contains(&input.lease_type.as_str()) {
            return Err(DbError::Conflict(format!(
                "invalid lease_type: {}",
                input.lease_type
            )));
        }
        let valid_freq = ["monthly", "quarterly", "annual"];
        if !valid_freq.contains(&input.payment_frequency.as_str()) {
            return Err(DbError::Conflict(format!(
                "invalid payment_frequency: {}",
                input.payment_frequency
            )));
        }

        let org = parse_uuid(org_id)?;
        let commencement = parse_date(&input.commencement_date)?;
        let end = parse_date(&input.end_date)?;

        if end <= commencement {
            return Err(DbError::Conflict(
                "end_date must be after commencement_date".into(),
            ));
        }

        let asset_acct = input
            .asset_account_id
            .as_deref()
            .map(parse_uuid)
            .transpose()?;
        let liab_acct = input
            .liability_account_id
            .as_deref()
            .map(parse_uuid)
            .transpose()?;
        let exp_acct = input
            .expense_account_id
            .as_deref()
            .map(parse_uuid)
            .transpose()?;

        // Compute PV = initial ROU asset = initial liability
        let num_months = months_between(commencement, end);
        let periods_per_year = match input.payment_frequency.as_str() {
            "quarterly" => 4,
            "annual" => 1,
            _ => 12,
        };
        let months_per_period = 12 / periods_per_year;
        let num_periods = num_months / months_per_period;
        let period_rate = input.discount_rate_bps as f64 / 10_000.0 / periods_per_year as f64;

        let pv = annuity_pv(input.payment_amount as f64, period_rate, num_periods);
        let initial = pv.round() as i64;

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO leases \
             (organization_id, name, description, lease_type, asset_account_id, \
              liability_account_id, expense_account_id, commencement_date, end_date, \
              payment_amount, payment_frequency, discount_rate_bps, \
              initial_rou_asset, initial_liability) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$13) RETURNING id",
        )
        .bind(org)
        .bind(&input.name)
        .bind(&input.description)
        .bind(&input.lease_type)
        .bind(asset_acct)
        .bind(liab_acct)
        .bind(exp_acct)
        .bind(commencement)
        .bind(end)
        .bind(input.payment_amount)
        .bind(&input.payment_frequency)
        .bind(input.discount_rate_bps)
        .bind(initial)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    /// Generate amortization schedule (posted + projected periods).
    pub async fn schedule(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<Vec<LeaseScheduleLine>, DbError> {
        let lease = Self::get_by_id(pool, org_id, id).await?;
        let lease_uuid = parse_uuid(id)?;

        // Fetch posted payments keyed by date.
        let posted: Vec<(Date, i64, i64, i64, i64)> = sqlx::query_as(
            "SELECT period_date, payment, interest, principal, rou_amort \
             FROM lease_payments WHERE lease_id = $1 ORDER BY period_date",
        )
        .bind(lease_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let posted_map: std::collections::HashMap<Date, (i64, i64, i64, i64)> = posted
            .into_iter()
            .map(|(d, pay, int, pri, amort)| (d, (pay, int, pri, amort)))
            .collect();

        let periods_per_year = match lease.payment_frequency.as_str() {
            "quarterly" => 4,
            "annual" => 1,
            _ => 12,
        };
        let months_per_period = 12 / periods_per_year;
        let num_months = months_between(lease.commencement_date, lease.end_date);
        let num_periods = num_months / months_per_period;
        let period_rate = lease.discount_rate_bps as f64 / 10_000.0 / periods_per_year as f64;

        let payment = lease.payment_amount as f64;
        let mut liability = lease.initial_liability as f64;
        let mut rou = lease.initial_rou_asset as f64;
        let rou_per_period = lease.initial_rou_asset as f64 / num_periods.max(1) as f64;

        let mut lines = Vec::with_capacity(num_periods as usize);
        let mut date = lease.commencement_date;

        for period in 1..=num_periods {
            let is_posted = posted_map.contains_key(&date);
            let (p_pay, p_int, p_pri, p_amort) =
                posted_map.get(&date).copied().unwrap_or((0, 0, 0, 0));

            let (interest, principal, rou_amort, expense): (f64, f64, f64, f64) = if is_posted {
                let expense = if lease.lease_type == "finance" {
                    p_int + p_amort
                } else {
                    p_pay
                };
                (p_int as f64, p_pri as f64, p_amort as f64, expense as f64)
            } else {
                let interest = liability * period_rate;
                let principal = payment - interest;
                let (rou_amort, expense) = if lease.lease_type == "finance" {
                    (rou_per_period, interest + rou_per_period)
                } else {
                    // Operating: straight-line expense = payment; ROU absorbs the difference
                    let amort = payment - interest;
                    (amort, payment)
                };
                (interest, principal, rou_amort, expense)
            };

            liability = (liability - principal).max(0.0);
            rou = (rou - rou_amort).max(0.0);

            lines.push(LeaseScheduleLine {
                period,
                period_date: date,
                payment: if is_posted {
                    p_pay
                } else {
                    lease.payment_amount
                },
                interest: interest.round() as i64,
                principal: principal.round() as i64,
                rou_amortization: rou_amort.round() as i64,
                lease_expense: expense.round() as i64,
                liability_balance: liability.round() as i64,
                rou_balance: rou.round() as i64,
                is_posted,
            });

            date = add_months(date, months_per_period);
        }

        Ok(lines)
    }

    /// Record an actual lease payment for a period (uses schedule values).
    pub async fn record_payment(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: RecordLeasePayment,
    ) -> Result<LeasePayment, DbError> {
        let period_date = parse_date(&input.period_date)?;
        let lease_uuid = parse_uuid(id)?;

        // Find the schedule line for this period.
        let schedule = Self::schedule(pool, org_id, id).await?;
        let line = schedule
            .iter()
            .find(|l| l.period_date == period_date)
            .ok_or_else(|| DbError::Conflict("period_date not in lease schedule".into()))?;

        if line.is_posted {
            return Err(DbError::Conflict(
                "payment already recorded for this period".into(),
            ));
        }

        let row: (
            Uuid,
            Uuid,
            Date,
            i64,
            i64,
            i64,
            i64,
            Option<String>,
            OffsetDateTime,
        ) = sqlx::query_as(
            "INSERT INTO lease_payments \
                 (lease_id, period_date, payment, interest, principal, rou_amort, notes) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7) \
                 RETURNING id, lease_id, period_date, payment, interest, principal, rou_amort, \
                           notes, created_at",
        )
        .bind(lease_uuid)
        .bind(period_date)
        .bind(line.payment)
        .bind(line.interest)
        .bind(line.principal)
        .bind(line.rou_amortization)
        .bind(&input.notes)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(LeasePayment {
            id: row.0.to_string(),
            lease_id: row.1.to_string(),
            period_date: row.2,
            payment: row.3,
            interest: row.4,
            principal: row.5,
            rou_amort: row.6,
            notes: row.7,
            created_at: row.8,
        })
    }

    pub async fn list_payments(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<Vec<LeasePayment>, DbError> {
        let lease_uuid = parse_uuid(id)?;
        // Verify ownership
        Self::get_by_id(pool, org_id, id).await?;

        #[allow(clippy::type_complexity)]
        let rows: Vec<(
            Uuid,
            Uuid,
            Date,
            i64,
            i64,
            i64,
            i64,
            Option<String>,
            OffsetDateTime,
        )> = sqlx::query_as(
            "SELECT id, lease_id, period_date, payment, interest, principal, rou_amort, \
                 notes, created_at FROM lease_payments WHERE lease_id = $1 ORDER BY period_date",
        )
        .bind(lease_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows
            .into_iter()
            .map(|r| LeasePayment {
                id: r.0.to_string(),
                lease_id: r.1.to_string(),
                period_date: r.2,
                payment: r.3,
                interest: r.4,
                principal: r.5,
                rou_amort: r.6,
                notes: r.7,
                created_at: r.8,
            })
            .collect())
    }

    pub async fn terminate(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: TerminateLease,
    ) -> Result<Lease, DbError> {
        let org = parse_uuid(org_id)?;
        let lid = parse_uuid(id)?;
        let terminated_at = parse_date(&input.terminated_at)?;

        let n = sqlx::query(
            "UPDATE leases SET status = 'terminated', terminated_at = $1, updated_at = NOW() \
             WHERE id = $2 AND organization_id = $3 AND status = 'active'",
        )
        .bind(terminated_at)
        .bind(lid)
        .bind(org)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if n == 0 {
            return Err(DbError::NotFound);
        }
        Self::get_by_id(pool, org_id, id).await
    }
}
