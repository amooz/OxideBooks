use oxidebooks_core::models::{
    CreatePortalAutopay, CreatePortalPaymentMethod, PortalAutopayEnrollment, PortalPaymentMethod,
};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct PmRow {
    id: Uuid,
    organization_id: Uuid,
    contact_id: Uuid,
    payment_type: String,
    provider: String,
    provider_token: String,
    last4: Option<String>,
    brand: Option<String>,
    exp_month: Option<i16>,
    exp_year: Option<i16>,
    bank_name: Option<String>,
    is_default: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<PmRow> for PortalPaymentMethod {
    fn from(r: PmRow) -> Self {
        PortalPaymentMethod {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            contact_id: r.contact_id.to_string(),
            payment_type: r.payment_type,
            provider: r.provider,
            provider_token: r.provider_token,
            last4: r.last4,
            brand: r.brand,
            exp_month: r.exp_month,
            exp_year: r.exp_year,
            bank_name: r.bank_name,
            is_default: r.is_default,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct AutopayRow {
    id: Uuid,
    organization_id: Uuid,
    contact_id: Uuid,
    payment_method_id: Uuid,
    is_active: bool,
    days_before_due: i32,
    max_amount: Option<i64>,
    enrolled_at: OffsetDateTime,
    cancelled_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<AutopayRow> for PortalAutopayEnrollment {
    fn from(r: AutopayRow) -> Self {
        PortalAutopayEnrollment {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            contact_id: r.contact_id.to_string(),
            payment_method_id: r.payment_method_id.to_string(),
            is_active: r.is_active,
            days_before_due: r.days_before_due,
            max_amount: r.max_amount,
            enrolled_at: r.enrolled_at,
            cancelled_at: r.cancelled_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

const PM_COLS: &str = "id, organization_id, contact_id, payment_type, provider, provider_token, \
    last4, brand, exp_month, exp_year, bank_name, is_default, created_at, updated_at";

const AP_COLS: &str = "id, organization_id, contact_id, payment_method_id, is_active, \
    days_before_due, max_amount, enrolled_at, cancelled_at, created_at, updated_at";

pub struct PortalPaymentMethodRepo;

impl PortalPaymentMethodRepo {
    pub async fn add(
        pool: &PgPool,
        org_id: &str,
        contact_id: &str,
        input: CreatePortalPaymentMethod,
    ) -> Result<PortalPaymentMethod, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = parse_uuid(contact_id)?;

        let valid_types = ["card", "bank_account", "paypal"];
        if !valid_types.contains(&input.payment_type.as_str()) {
            return Err(DbError::Conflict(format!(
                "payment_type must be one of: {}",
                valid_types.join(", ")
            )));
        }

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        if input.is_default {
            sqlx::query(
                "UPDATE portal_payment_methods SET is_default = FALSE, updated_at = NOW() \
                 WHERE organization_id = $1 AND contact_id = $2 AND is_default = TRUE",
            )
            .bind(org_uuid)
            .bind(contact_uuid)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        // If this is the first payment method, make it default regardless
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM portal_payment_methods \
             WHERE organization_id = $1 AND contact_id = $2",
        )
        .bind(org_uuid)
        .bind(contact_uuid)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        let is_default = input.is_default || count == 0;

        let row: PmRow = sqlx::query_as(&format!(
            "INSERT INTO portal_payment_methods \
             (organization_id, contact_id, payment_type, provider_token, last4, brand, \
              exp_month, exp_year, bank_name, is_default) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) \
             RETURNING {PM_COLS}"
        ))
        .bind(org_uuid)
        .bind(contact_uuid)
        .bind(&input.payment_type)
        .bind(&input.provider_token)
        .bind(&input.last4)
        .bind(&input.brand)
        .bind(input.exp_month)
        .bind(input.exp_year)
        .bind(&input.bank_name)
        .bind(is_default)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(row.into())
    }

    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        contact_id: &str,
    ) -> Result<Vec<PortalPaymentMethod>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = parse_uuid(contact_id)?;

        let rows: Vec<PmRow> = sqlx::query_as(&format!(
            "SELECT {PM_COLS} FROM portal_payment_methods \
             WHERE organization_id = $1 AND contact_id = $2 \
             ORDER BY is_default DESC, created_at DESC"
        ))
        .bind(org_uuid)
        .bind(contact_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get(
        pool: &PgPool,
        org_id: &str,
        contact_id: &str,
        id: &str,
    ) -> Result<PortalPaymentMethod, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = parse_uuid(contact_id)?;
        let pm_uuid = parse_uuid(id)?;

        let row: Option<PmRow> = sqlx::query_as(&format!(
            "SELECT {PM_COLS} FROM portal_payment_methods \
             WHERE id = $1 AND organization_id = $2 AND contact_id = $3"
        ))
        .bind(pm_uuid)
        .bind(org_uuid)
        .bind(contact_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        row.map(Into::into).ok_or(DbError::NotFound)
    }

    pub async fn set_default(
        pool: &PgPool,
        org_id: &str,
        contact_id: &str,
        id: &str,
    ) -> Result<PortalPaymentMethod, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = parse_uuid(contact_id)?;
        let pm_uuid = parse_uuid(id)?;

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        // Set target to default first — if it doesn't exist, roll back without
        // touching any other rows.
        let row: Option<PmRow> = sqlx::query_as(&format!(
            "UPDATE portal_payment_methods SET is_default = TRUE, updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2 AND contact_id = $3 \
             RETURNING {PM_COLS}"
        ))
        .bind(pm_uuid)
        .bind(org_uuid)
        .bind(contact_uuid)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        let row = match row {
            Some(r) => r,
            None => {
                tx.rollback().await.map_err(map_sqlx_err)?;
                return Err(DbError::NotFound);
            }
        };

        // Clear every other default for this contact.
        sqlx::query(
            "UPDATE portal_payment_methods SET is_default = FALSE, updated_at = NOW() \
             WHERE organization_id = $1 AND contact_id = $2 AND id <> $3",
        )
        .bind(org_uuid)
        .bind(contact_uuid)
        .bind(pm_uuid)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(row.into())
    }

    pub async fn delete(
        pool: &PgPool,
        org_id: &str,
        contact_id: &str,
        id: &str,
    ) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = parse_uuid(contact_id)?;
        let pm_uuid = parse_uuid(id)?;

        let result = sqlx::query(
            "DELETE FROM portal_payment_methods \
             WHERE id = $1 AND organization_id = $2 AND contact_id = $3",
        )
        .bind(pm_uuid)
        .bind(org_uuid)
        .bind(contact_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    // Autopay methods

    pub async fn enroll_autopay(
        pool: &PgPool,
        org_id: &str,
        contact_id: &str,
        input: CreatePortalAutopay,
    ) -> Result<PortalAutopayEnrollment, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = parse_uuid(contact_id)?;
        let pm_uuid = parse_uuid(&input.payment_method_id)?;

        // Verify payment method belongs to this contact
        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        // Verify payment method belongs to this contact (inside tx for consistency).
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM portal_payment_methods \
             WHERE id = $1 AND organization_id = $2 AND contact_id = $3)",
        )
        .bind(pm_uuid)
        .bind(org_uuid)
        .bind(contact_uuid)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        if !exists {
            return Err(DbError::NotFound);
        }

        // Cancel any existing active enrollment atomically with the new insert.
        sqlx::query(
            "UPDATE portal_autopay_enrollments \
             SET is_active = FALSE, cancelled_at = NOW(), updated_at = NOW() \
             WHERE organization_id = $1 AND contact_id = $2 AND is_active = TRUE",
        )
        .bind(org_uuid)
        .bind(contact_uuid)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        let row: AutopayRow = sqlx::query_as(&format!(
            "INSERT INTO portal_autopay_enrollments \
             (organization_id, contact_id, payment_method_id, days_before_due, max_amount) \
             VALUES ($1,$2,$3,$4,$5) RETURNING {AP_COLS}"
        ))
        .bind(org_uuid)
        .bind(contact_uuid)
        .bind(pm_uuid)
        .bind(input.days_before_due)
        .bind(input.max_amount)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(row.into())
    }

    pub async fn get_autopay(
        pool: &PgPool,
        org_id: &str,
        contact_id: &str,
    ) -> Result<Option<PortalAutopayEnrollment>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = parse_uuid(contact_id)?;

        let row: Option<AutopayRow> = sqlx::query_as(&format!(
            "SELECT {AP_COLS} FROM portal_autopay_enrollments \
             WHERE organization_id = $1 AND contact_id = $2 AND is_active = TRUE"
        ))
        .bind(org_uuid)
        .bind(contact_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.map(Into::into))
    }

    pub async fn cancel_autopay(
        pool: &PgPool,
        org_id: &str,
        contact_id: &str,
    ) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = parse_uuid(contact_id)?;

        let result = sqlx::query(
            "UPDATE portal_autopay_enrollments \
             SET is_active = FALSE, cancelled_at = NOW(), updated_at = NOW() \
             WHERE organization_id = $1 AND contact_id = $2 AND is_active = TRUE",
        )
        .bind(org_uuid)
        .bind(contact_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }
}
