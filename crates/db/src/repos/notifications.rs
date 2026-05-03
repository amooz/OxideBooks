use oxidebooks_core::models::{CreateNotification, Notification};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct NotificationRow {
    id: Uuid,
    organization_id: Uuid,
    user_id: Uuid,
    notification_type: String,
    title: String,
    body: String,
    entity_type: Option<String>,
    entity_id: Option<Uuid>,
    is_read: bool,
    created_at: OffsetDateTime,
}

fn from_row(r: NotificationRow) -> Notification {
    Notification {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        user_id: r.user_id.to_string(),
        notification_type: r.notification_type,
        title: r.title,
        body: r.body,
        entity_type: r.entity_type,
        entity_id: r.entity_id.map(|u| u.to_string()),
        is_read: r.is_read,
        created_at: r.created_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

pub struct NotificationRepo;

impl NotificationRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        user_id: &str,
        unread_only: bool,
    ) -> Result<Vec<Notification>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let user_uuid = parse_uuid(user_id)?;
        let rows: Vec<NotificationRow> = if unread_only {
            sqlx::query_as(
                "SELECT id, organization_id, user_id, notification_type, title, body, \
                 entity_type, entity_id, is_read, created_at \
                 FROM notifications \
                 WHERE organization_id = $1 AND user_id = $2 AND is_read = false \
                 ORDER BY created_at DESC LIMIT 100",
            )
            .bind(org_uuid)
            .bind(user_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(
                "SELECT id, organization_id, user_id, notification_type, title, body, \
                 entity_type, entity_id, is_read, created_at \
                 FROM notifications \
                 WHERE organization_id = $1 AND user_id = $2 \
                 ORDER BY created_at DESC LIMIT 100",
            )
            .bind(org_uuid)
            .bind(user_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        };
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateNotification,
    ) -> Result<Notification, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let user_uuid = parse_uuid(&input.user_id)?;
        let entity_uuid = input.entity_id.as_deref().map(parse_uuid).transpose()?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO notifications \
             (organization_id, user_id, notification_type, title, body, entity_type, entity_id) \
             VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id",
        )
        .bind(org_uuid)
        .bind(user_uuid)
        .bind(&input.notification_type)
        .bind(&input.title)
        .bind(&input.body)
        .bind(&input.entity_type)
        .bind(entity_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: NotificationRow = sqlx::query_as(
            "SELECT id, organization_id, user_id, notification_type, title, body, \
             entity_type, entity_id, is_read, created_at \
             FROM notifications WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(from_row(row))
    }

    pub async fn mark_read(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n = sqlx::query(
            "UPDATE notifications SET is_read = true WHERE id = $1 AND organization_id = $2",
        )
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

    pub async fn mark_all_read(pool: &PgPool, org_id: &str, user_id: &str) -> Result<u64, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let user_uuid = parse_uuid(user_id)?;
        let n = sqlx::query(
            "UPDATE notifications SET is_read = true \
             WHERE organization_id = $1 AND user_id = $2 AND is_read = false",
        )
        .bind(org_uuid)
        .bind(user_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        Ok(n)
    }
}
