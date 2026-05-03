use oxidebooks_core::models::{
    CreateCustomFieldDefinition, CustomFieldDefinition, CustomFieldValue, SetCustomFieldValue,
    UpdateCustomFieldDefinition,
};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct DefRow {
    id: Uuid,
    organization_id: Uuid,
    entity_type: String,
    name: String,
    field_type: String,
    is_required: bool,
    sort_order: i32,
    created_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct ValueRow {
    definition_id: Uuid,
    entity_id: Uuid,
    name: String,
    field_type: String,
    value: Option<String>,
}

fn def_from_row(r: DefRow) -> CustomFieldDefinition {
    CustomFieldDefinition {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        entity_type: r.entity_type,
        name: r.name,
        field_type: r.field_type,
        is_required: r.is_required,
        sort_order: r.sort_order,
        created_at: r.created_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

pub struct CustomFieldRepo;

impl CustomFieldRepo {
    pub async fn list_definitions(
        pool: &PgPool,
        org_id: &str,
        entity_type: Option<&str>,
    ) -> Result<Vec<CustomFieldDefinition>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<DefRow> = sqlx::query_as(
            "SELECT id, organization_id, entity_type, name, field_type, is_required, sort_order, created_at \
             FROM custom_field_definitions \
             WHERE organization_id = $1 AND ($2::text IS NULL OR entity_type = $2) \
             ORDER BY entity_type, sort_order, name",
        )
        .bind(org_uuid)
        .bind(entity_type)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(def_from_row).collect())
    }

    pub async fn get_definition(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<CustomFieldDefinition, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: DefRow = sqlx::query_as(
            "SELECT id, organization_id, entity_type, name, field_type, is_required, sort_order, created_at \
             FROM custom_field_definitions WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(def_from_row(row))
    }

    pub async fn create_definition(
        pool: &PgPool,
        org_id: &str,
        input: CreateCustomFieldDefinition,
    ) -> Result<CustomFieldDefinition, DbError> {
        let valid_entity_types = ["contact", "invoice", "expense", "project"];
        let valid_field_types = ["text", "number", "date", "boolean"];
        if !valid_entity_types.contains(&input.entity_type.as_str()) {
            return Err(DbError::Conflict(format!(
                "invalid entity_type: {}",
                input.entity_type
            )));
        }
        if !valid_field_types.contains(&input.field_type.as_str()) {
            return Err(DbError::Conflict(format!(
                "invalid field_type: {}",
                input.field_type
            )));
        }
        let org_uuid = parse_uuid(org_id)?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO custom_field_definitions \
             (organization_id, entity_type, name, field_type, is_required, sort_order) \
             VALUES ($1,$2,$3,$4,$5,$6) RETURNING id",
        )
        .bind(org_uuid)
        .bind(&input.entity_type)
        .bind(&input.name)
        .bind(&input.field_type)
        .bind(input.is_required)
        .bind(input.sort_order)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Self::get_definition(pool, org_id, &id.to_string()).await
    }

    pub async fn update_definition(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateCustomFieldDefinition,
    ) -> Result<CustomFieldDefinition, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n = sqlx::query(
            "UPDATE custom_field_definitions SET \
             name        = COALESCE($1, name), \
             is_required = COALESCE($2, is_required), \
             sort_order  = COALESCE($3, sort_order) \
             WHERE id = $4 AND organization_id = $5",
        )
        .bind(input.name)
        .bind(input.is_required)
        .bind(input.sort_order)
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::NotFound);
        }
        Self::get_definition(pool, org_id, id).await
    }

    pub async fn delete_definition(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n = sqlx::query(
            "DELETE FROM custom_field_definitions WHERE id = $1 AND organization_id = $2",
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

    pub async fn get_values(
        pool: &PgPool,
        org_id: &str,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Vec<CustomFieldValue>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let entity_uuid = parse_uuid(entity_id)?;
        let rows: Vec<ValueRow> = sqlx::query_as(
            "SELECT v.definition_id, v.entity_id, d.name, d.field_type, v.value \
             FROM custom_field_definitions d \
             LEFT JOIN custom_field_values v \
               ON v.definition_id = d.id AND v.entity_id = $3 \
             WHERE d.organization_id = $1 AND d.entity_type = $2 \
             ORDER BY d.sort_order, d.name",
        )
        .bind(org_uuid)
        .bind(entity_type)
        .bind(entity_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows
            .into_iter()
            .map(|r| CustomFieldValue {
                definition_id: r.definition_id.to_string(),
                entity_id: r.entity_id.to_string(),
                name: r.name,
                field_type: r.field_type,
                value: r.value,
            })
            .collect())
    }

    pub async fn set_values(
        pool: &PgPool,
        org_id: &str,
        entity_id: &str,
        values: Vec<SetCustomFieldValue>,
    ) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let entity_uuid = parse_uuid(entity_id)?;
        for v in values {
            let def_uuid = parse_uuid(&v.definition_id)?;
            // Verify the definition belongs to this org
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM custom_field_definitions WHERE id = $1 AND organization_id = $2)",
            )
            .bind(def_uuid)
            .bind(org_uuid)
            .fetch_one(pool)
            .await
            .map_err(map_sqlx_err)?;
            if !exists {
                return Err(DbError::NotFound);
            }
            sqlx::query(
                "INSERT INTO custom_field_values (definition_id, entity_id, value, updated_at) \
                 VALUES ($1,$2,$3,NOW()) \
                 ON CONFLICT (definition_id, entity_id) DO UPDATE SET value = EXCLUDED.value, updated_at = NOW()",
            )
            .bind(def_uuid)
            .bind(entity_uuid)
            .bind(&v.value)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;
        }
        Ok(())
    }
}
