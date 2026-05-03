use oxidebooks_core::models::{
    CreateLeaveRequest, CreateLeaveType, LeaveBalance, LeaveRequest, LeaveType, UpdateLeaveType,
};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct LeaveTypeRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    days_per_year: f64,
    is_paid: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<LeaveTypeRow> for LeaveType {
    fn from(r: LeaveTypeRow) -> Self {
        LeaveType {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            name: r.name,
            days_per_year: r.days_per_year,
            is_paid: r.is_paid,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct LeaveRequestRow {
    id: Uuid,
    organization_id: Uuid,
    employee_id: Uuid,
    leave_type_id: Uuid,
    start_date: Date,
    end_date: Date,
    days: f64,
    status: String,
    notes: Option<String>,
    approved_by: Option<Uuid>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<LeaveRequestRow> for LeaveRequest {
    fn from(r: LeaveRequestRow) -> Self {
        LeaveRequest {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            employee_id: r.employee_id.to_string(),
            leave_type_id: r.leave_type_id.to_string(),
            start_date: r.start_date,
            end_date: r.end_date,
            days: r.days,
            status: r.status,
            notes: r.notes,
            approved_by: r.approved_by.map(|u| u.to_string()),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

pub struct LeaveRepo;

impl LeaveRepo {
    // ── Leave Types ───────────────────────────────────────────────────────────

    pub async fn list_types(pool: &PgPool, org_id: &str) -> Result<Vec<LeaveType>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<LeaveTypeRow> = sqlx::query_as(
            "SELECT id, organization_id, name, days_per_year, is_paid, created_at, updated_at \
             FROM leave_types WHERE organization_id = $1 ORDER BY name ASC",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(LeaveType::from).collect())
    }

    pub async fn create_type(
        pool: &PgPool,
        org_id: &str,
        input: CreateLeaveType,
    ) -> Result<LeaveType, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO leave_types (id, organization_id, name, days_per_year, is_paid) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(&input.name)
        .bind(input.days_per_year)
        .bind(input.is_paid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;
        let row: LeaveTypeRow = sqlx::query_as(
            "SELECT id, organization_id, name, days_per_year, is_paid, created_at, updated_at \
             FROM leave_types WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(row.into())
    }

    pub async fn update_type(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateLeaveType,
    ) -> Result<LeaveType, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows = sqlx::query(
            "UPDATE leave_types SET \
             name          = COALESCE($3, name), \
             days_per_year = COALESCE($4, days_per_year), \
             is_paid       = COALESCE($5, is_paid), \
             updated_at    = NOW() \
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .bind(&input.name)
        .bind(input.days_per_year)
        .bind(input.is_paid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if rows == 0 {
            return Err(DbError::NotFound);
        }
        let row: LeaveTypeRow = sqlx::query_as(
            "SELECT id, organization_id, name, days_per_year, is_paid, created_at, updated_at \
             FROM leave_types WHERE id = $1",
        )
        .bind(id_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(row.into())
    }

    pub async fn delete_type(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows = sqlx::query("DELETE FROM leave_types WHERE organization_id = $1 AND id = $2")
            .bind(org_uuid)
            .bind(id_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?
            .rows_affected();
        if rows == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    // ── Leave Requests ────────────────────────────────────────────────────────

    pub async fn list_requests(
        pool: &PgPool,
        org_id: &str,
        employee_id: Option<&str>,
    ) -> Result<Vec<LeaveRequest>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<LeaveRequestRow> = if let Some(eid) = employee_id {
            let emp_uuid = parse_uuid(eid)?;
            sqlx::query_as(
                "SELECT id, organization_id, employee_id, leave_type_id, start_date, end_date, \
                 days, status, notes, approved_by, created_at, updated_at \
                 FROM leave_requests WHERE organization_id = $1 AND employee_id = $2 \
                 ORDER BY start_date DESC",
            )
            .bind(org_uuid)
            .bind(emp_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(
                "SELECT id, organization_id, employee_id, leave_type_id, start_date, end_date, \
                 days, status, notes, approved_by, created_at, updated_at \
                 FROM leave_requests WHERE organization_id = $1 \
                 ORDER BY start_date DESC",
            )
            .bind(org_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        };
        Ok(rows.into_iter().map(LeaveRequest::from).collect())
    }

    pub async fn create_request(
        pool: &PgPool,
        org_id: &str,
        input: CreateLeaveRequest,
    ) -> Result<LeaveRequest, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let emp_uuid = parse_uuid(&input.employee_id)?;
        let lt_uuid = parse_uuid(&input.leave_type_id)?;

        if input.days <= 0.0 {
            return Err(DbError::Conflict("days must be positive".into()));
        }
        if input.end_date < input.start_date {
            return Err(DbError::Conflict("end_date must be >= start_date".into()));
        }

        // Verify employee and leave_type belong to org.
        let emp_exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM employees WHERE organization_id = $1 AND id = $2")
                .bind(org_uuid)
                .bind(emp_uuid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;
        if emp_exists.is_none() {
            return Err(DbError::Conflict("employee not found".into()));
        }

        let lt_exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM leave_types WHERE organization_id = $1 AND id = $2")
                .bind(org_uuid)
                .bind(lt_uuid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;
        if lt_exists.is_none() {
            return Err(DbError::Conflict("leave type not found".into()));
        }

        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO leave_requests \
             (id, organization_id, employee_id, leave_type_id, start_date, end_date, days, notes) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(emp_uuid)
        .bind(lt_uuid)
        .bind(input.start_date)
        .bind(input.end_date)
        .bind(input.days)
        .bind(&input.notes)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: LeaveRequestRow = sqlx::query_as(
            "SELECT id, organization_id, employee_id, leave_type_id, start_date, end_date, \
             days, status, notes, approved_by, created_at, updated_at \
             FROM leave_requests WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(row.into())
    }

    pub async fn update_request_status(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        new_status: &str,
        approver_id: Option<&str>,
    ) -> Result<LeaveRequest, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let approver_uuid = approver_id.map(parse_uuid).transpose()?;

        let valid = ["approved", "rejected", "cancelled"];
        if !valid.contains(&new_status) {
            return Err(DbError::Conflict(format!(
                "status must be one of: {}",
                valid.join(", ")
            )));
        }

        let approved_at: Option<OffsetDateTime> = if new_status == "approved" {
            Some(time::OffsetDateTime::now_utc())
        } else {
            None
        };

        let rows = sqlx::query(
            "UPDATE leave_requests SET \
             status = $1, approved_by = $2, approved_at = $3, updated_at = NOW() \
             WHERE organization_id = $4 AND id = $5 AND status = 'pending'",
        )
        .bind(new_status)
        .bind(approver_uuid)
        .bind(approved_at)
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows == 0 {
            return Err(DbError::Conflict(
                "request not found or not in pending status".into(),
            ));
        }

        let row: LeaveRequestRow = sqlx::query_as(
            "SELECT id, organization_id, employee_id, leave_type_id, start_date, end_date, \
             days, status, notes, approved_by, created_at, updated_at \
             FROM leave_requests WHERE id = $1",
        )
        .bind(id_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(row.into())
    }

    pub async fn employee_balance(
        pool: &PgPool,
        org_id: &str,
        employee_id: &str,
    ) -> Result<Vec<LeaveBalance>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let emp_uuid = parse_uuid(employee_id)?;

        #[derive(sqlx::FromRow)]
        struct BalanceRow {
            leave_type_id: Uuid,
            leave_type_name: String,
            days_per_year: f64,
            days_taken: f64,
            days_pending: f64,
        }

        let rows: Vec<BalanceRow> = sqlx::query_as(
            "SELECT lt.id AS leave_type_id, lt.name AS leave_type_name, \
             lt.days_per_year, \
             COALESCE(SUM(lr.days) FILTER (WHERE lr.status = 'approved'), 0.0) AS days_taken, \
             COALESCE(SUM(lr.days) FILTER (WHERE lr.status = 'pending'),  0.0) AS days_pending \
             FROM leave_types lt \
             LEFT JOIN leave_requests lr \
               ON lr.leave_type_id = lt.id \
               AND lr.employee_id = $2 \
               AND lr.organization_id = $1 \
             WHERE lt.organization_id = $1 \
             GROUP BY lt.id, lt.name, lt.days_per_year \
             ORDER BY lt.name ASC",
        )
        .bind(org_uuid)
        .bind(emp_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows
            .into_iter()
            .map(|r| LeaveBalance {
                leave_type_id: r.leave_type_id.to_string(),
                leave_type_name: r.leave_type_name,
                days_per_year: r.days_per_year,
                days_taken: r.days_taken,
                days_pending: r.days_pending,
                days_remaining: (r.days_per_year - r.days_taken - r.days_pending).max(0.0),
            })
            .collect())
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
