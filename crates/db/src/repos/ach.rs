use oxidebooks_core::models::{
    AchPayment, CollectAch, GenerateNachaRequest, NachaFile, PayBillAch,
};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

fn parse_date(s: &str) -> Result<Date, DbError> {
    let fmt = time::macros::format_description!("[year]-[month]-[day]");
    Date::parse(s, fmt).map_err(|_| DbError::Conflict(format!("invalid date: {s}")))
}

fn next_business_day() -> Date {
    let mut d = OffsetDateTime::now_utc().date();
    d = d.next_day().unwrap_or(d);
    // Advance past Saturday (6) and Sunday (7)
    loop {
        let dow = d.weekday().number_days_from_monday();
        if dow < 5 {
            break;
        }
        d = d.next_day().unwrap_or(d);
    }
    d
}

#[derive(sqlx::FromRow)]
struct AchRow {
    id: Uuid,
    organization_id: Uuid,
    entry_type: String,
    invoice_id: Option<Uuid>,
    bill_id: Option<Uuid>,
    routing_number: String,
    account_number: String,
    account_type: String,
    amount: i64,
    status: String,
    trace_number: Option<String>,
    effective_date: Date,
    return_code: Option<String>,
    submitted_at: Option<OffsetDateTime>,
    settled_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<AchRow> for AchPayment {
    fn from(r: AchRow) -> Self {
        AchPayment {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            entry_type: r.entry_type,
            invoice_id: r.invoice_id.map(|u| u.to_string()),
            bill_id: r.bill_id.map(|u| u.to_string()),
            routing_number: r.routing_number,
            account_number: r.account_number,
            account_type: r.account_type,
            amount: r.amount,
            status: r.status,
            trace_number: r.trace_number,
            effective_date: r.effective_date,
            return_code: r.return_code,
            submitted_at: r.submitted_at,
            settled_at: r.settled_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

const ACH_COLS: &str = "id, organization_id, entry_type::TEXT, invoice_id, bill_id, \
     routing_number, account_number, account_type, amount, status::TEXT, trace_number, \
     effective_date, return_code, submitted_at, settled_at, created_at, updated_at";

// ─── NACHA builder ────────────────────────────────────────────────────────────

struct NachaEntry {
    routing: String,
    account: String,
    amount: i64,
    name: String,
    is_debit: bool,
    payment_id: String,
}

fn build_nacha(
    entries: &[NachaEntry],
    company_name: &str,
    company_id: &str,
    orig_routing: &str,
    effective_date: &Date,
) -> String {
    let fmt_date = time::macros::format_description!("[year repr:last_two][month][day]");
    let eff_str = effective_date.format(fmt_date).unwrap_or_default();
    let now = OffsetDateTime::now_utc();
    let create_date = now.date().format(fmt_date).unwrap_or_default();
    let create_time = format!("{:02}{:02}", now.hour(), now.minute());

    let company_name_padded = format!("{:<16}", &company_name[..company_name.len().min(16)]);
    let company_id_padded = format!("{:<10}", &company_id[..company_id.len().min(10)]);
    let orig_routing_padded = format!("{:0>8}", &orig_routing[..orig_routing.len().min(8)]);

    let mut lines = Vec::new();

    // File Header (Record Type 1)
    lines.push(format!(
        "1{:0>2}{create_date}{create_time}A094101{orig_routing_padded}{company_name_padded}{company_id_padded}PPD{eff_str}   1",
        1,
        orig_routing_padded = orig_routing_padded,
        company_name_padded = company_name_padded,
        company_id_padded = company_id_padded,
        eff_str = eff_str,
        create_date = create_date,
        create_time = create_time,
    ));

    // Batch Header (Record Type 5)
    lines.push(format!(
        "5200{:<16}{:<10}PPD{:<10}{eff_str}   1{orig_routing_padded}0000001",
        company_name_padded.trim(),
        company_id_padded.trim(),
        "PAYMENT   ",
        eff_str = eff_str,
        orig_routing_padded = orig_routing_padded,
    ));

    let mut debit_total: i64 = 0;
    let mut credit_total: i64 = 0;
    let mut entry_count = 0usize;
    let mut hash_sum: u64 = 0;

    for (i, e) in entries.iter().enumerate() {
        let txn_code = if e.is_debit { "27" } else { "22" };
        let routing_check: u64 = e.routing[..8.min(e.routing.len())].parse().unwrap_or(0);
        hash_sum += routing_check;

        let name_padded = format!("{:<22}", &e.name[..e.name.len().min(22)]);
        let acct_padded = format!("{:<17}", &e.account[..e.account.len().min(17)]);
        let trace = format!(
            "{orig_routing_padded}{:0>7}",
            i + 1,
            orig_routing_padded = orig_routing_padded
        );

        // Record Type 6 — Entry Detail
        lines.push(format!(
            "6{txn_code}{:0>9}{acct_padded}{:010}{name_padded}  0{trace}",
            routing_check,
            e.amount,
            txn_code = txn_code,
            acct_padded = acct_padded,
            name_padded = name_padded,
            trace = trace,
        ));

        if e.is_debit {
            debit_total += e.amount;
        } else {
            credit_total += e.amount;
        }
        entry_count += 1;
    }

    // Batch Control (Record Type 8)
    lines.push(format!(
        "8200{:0>6}{:010}{:010}{:0>12}{:<39}{:0>8}000001",
        entry_count,
        debit_total,
        credit_total,
        hash_sum % 10_000_000_000,
        company_id_padded.trim(),
        orig_routing_padded,
    ));

    // File Control (Record Type 9)
    let block_count = ((lines.len() + 2) as f64 / 10.0).ceil() as usize;
    lines.push(format!(
        "9{:0>6}{:0>8}{:0>10}{:010}{:010}{:0>12}{:>39}",
        1,
        block_count,
        entry_count,
        debit_total,
        credit_total,
        hash_sum % 10_000_000_000,
        "",
    ));

    // Pad to multiple of 10 lines with "9" records
    while lines.len() % 10 != 0 {
        lines.push(format!("{:0>94}", "9"));
    }

    lines.join("\n")
}

// ─── Repo ─────────────────────────────────────────────────────────────────────

pub struct AchRepo;

impl AchRepo {
    pub async fn collect_ach(
        pool: &PgPool,
        org_id: &str,
        invoice_id: &str,
        input: CollectAch,
    ) -> Result<AchPayment, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let inv_uuid = parse_uuid(invoice_id)?;

        // Fetch invoice amount due.
        let (amount, currency): (Option<i64>, String) = {
            #[derive(sqlx::FromRow)]
            struct AmtRow {
                total: Option<i64>,
                currency: String,
            }
            let row: AmtRow = sqlx::query_as(
                "SELECT \
                    (SELECT COALESCE(SUM(quantity * unit_price / 100), 0) \
                     FROM invoice_lines WHERE invoice_id = i.id)::BIGINT AS total, \
                    i.currency \
                 FROM invoices i \
                 WHERE i.id = $1 AND i.organization_id = $2 AND i.invoice_type = 'invoice'",
            )
            .bind(inv_uuid)
            .bind(org_uuid)
            .fetch_optional(pool)
            .await
            .map_err(map_sqlx_err)?
            .ok_or(DbError::NotFound)?;
            (row.total, row.currency)
        };
        let _ = currency;

        let amount = amount.unwrap_or(0);
        if amount <= 0 {
            return Err(DbError::Conflict("invoice has no amount due".into()));
        }

        let effective = match &input.effective_date {
            Some(s) => parse_date(s)?,
            None => next_business_day(),
        };

        let trace = format!(
            "{:015}",
            Uuid::new_v4().as_u128() % 1_000_000_000_000_000u128
        );

        let row: AchRow = sqlx::query_as(&format!(
            "INSERT INTO ach_payments \
             (organization_id, entry_type, invoice_id, routing_number, account_number, \
              account_type, amount, status, trace_number, effective_date) \
             VALUES ($1, 'collection', $2, $3, $4, $5, $6, 'submitted', $7, $8) \
             RETURNING {ACH_COLS}"
        ))
        .bind(org_uuid)
        .bind(inv_uuid)
        .bind(&input.routing_number)
        .bind(&input.account_number)
        .bind(&input.account_type)
        .bind(amount)
        .bind(&trace)
        .bind(effective)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.into())
    }

    pub async fn pay_bill_ach(
        pool: &PgPool,
        org_id: &str,
        bill_id: &str,
        input: PayBillAch,
    ) -> Result<AchPayment, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let bill_uuid = parse_uuid(bill_id)?;

        let amount: Option<i64> = sqlx::query_scalar(
            "SELECT \
                (SELECT COALESCE(SUM(quantity * unit_price / 100), 0) \
                 FROM bill_lines WHERE bill_id = vb.id)::BIGINT \
             FROM vendor_bills vb \
             WHERE vb.id = $1 AND vb.organization_id = $2",
        )
        .bind(bill_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        let amount = amount.unwrap_or(0);
        if amount <= 0 {
            return Err(DbError::Conflict("bill has no amount due".into()));
        }

        let effective = match &input.effective_date {
            Some(s) => parse_date(s)?,
            None => next_business_day(),
        };

        let trace = format!(
            "{:015}",
            Uuid::new_v4().as_u128() % 1_000_000_000_000_000u128
        );

        let row: AchRow = sqlx::query_as(&format!(
            "INSERT INTO ach_payments \
             (organization_id, entry_type, bill_id, routing_number, account_number, \
              account_type, amount, status, trace_number, effective_date) \
             VALUES ($1, 'payment', $2, $3, $4, $5, $6, 'submitted', $7, $8) \
             RETURNING {ACH_COLS}"
        ))
        .bind(org_uuid)
        .bind(bill_uuid)
        .bind(&input.routing_number)
        .bind(&input.account_number)
        .bind(&input.account_type)
        .bind(amount)
        .bind(&trace)
        .bind(effective)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.into())
    }

    pub async fn generate_nacha(
        pool: &PgPool,
        org_id: &str,
        input: GenerateNachaRequest,
    ) -> Result<NachaFile, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let effective = match &input.effective_date {
            Some(s) => parse_date(s)?,
            None => next_business_day(),
        };

        let rows: Vec<AchRow> = sqlx::query_as(&format!(
            "SELECT {ACH_COLS} FROM ach_payments \
             WHERE organization_id = $1 AND status = 'pending' AND effective_date = $2 \
             ORDER BY created_at ASC"
        ))
        .bind(org_uuid)
        .bind(effective)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        if rows.is_empty() {
            return Err(DbError::Conflict(
                "no pending ACH payments for this effective date".into(),
            ));
        }

        let company_name = input.company_name.as_deref().unwrap_or("OXIDEBOOKS");
        let company_id = input.company_id.as_deref().unwrap_or("0000000000");
        let orig_routing = input.originating_routing.as_deref().unwrap_or("000000000");

        let entries: Vec<NachaEntry> = rows
            .iter()
            .map(|r| NachaEntry {
                routing: r.routing_number.clone(),
                account: r.account_number.clone(),
                amount: r.amount,
                name: format!("PAYMENT {}", &r.id.to_string()[..8]),
                is_debit: r.entry_type == "collection",
                payment_id: r.id.to_string(),
            })
            .collect();

        let total_debit: i64 = entries
            .iter()
            .filter(|e| e.is_debit)
            .map(|e| e.amount)
            .sum();
        let total_credit: i64 = entries
            .iter()
            .filter(|e| !e.is_debit)
            .map(|e| e.amount)
            .sum();
        let payment_ids: Vec<String> = entries.iter().map(|e| e.payment_id.clone()).collect();
        let entry_count = entries.len();

        let nacha_text = build_nacha(&entries, company_name, company_id, orig_routing, &effective);

        // Mark payments as submitted.
        let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
        sqlx::query(
            "UPDATE ach_payments SET status = 'submitted', submitted_at = NOW(), updated_at = NOW() \
             WHERE id = ANY($1)",
        )
        .bind(&ids)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(NachaFile {
            nacha_text,
            entry_count,
            total_debit,
            total_credit,
            payment_ids,
        })
    }

    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<AchPayment>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<AchRow> = sqlx::query_as(&format!(
            "SELECT {ACH_COLS} FROM ach_payments \
             WHERE organization_id = $1 ORDER BY created_at DESC"
        ))
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(AchPayment::from).collect())
    }
}
