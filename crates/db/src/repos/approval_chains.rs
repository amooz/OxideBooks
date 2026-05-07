use oxidebooks_core::models::{
    ApprovalChain, ApprovalChainStep, ApprovalDecision, ApprovalRequest, CreateApprovalChain,
    RecordApprovalDecision, SubmitApprovalRequest,
};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct ChainRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    entity_type: String,
    description: Option<String>,
    is_active: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct StepRow {
    id: Uuid,
    chain_id: Uuid,
    step_order: i32,
    required_role: String,
    approver_user_id: Option<Uuid>,
    require_all: bool,
}

#[derive(sqlx::FromRow)]
struct RequestRow {
    id: Uuid,
    organization_id: Uuid,
    chain_id: Uuid,
    entity_type: String,
    entity_id: Uuid,
    status: String,
    current_step: i32,
    requested_by: Option<Uuid>,
    notes: Option<String>,
    completed_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct DecisionRow {
    id: Uuid,
    request_id: Uuid,
    step_order: i32,
    approver_user_id: Option<Uuid>,
    decision: String,
    notes: Option<String>,
    decided_at: OffsetDateTime,
}

impl From<StepRow> for ApprovalChainStep {
    fn from(r: StepRow) -> Self {
        ApprovalChainStep {
            id: r.id.to_string(),
            chain_id: r.chain_id.to_string(),
            step_order: r.step_order,
            required_role: r.required_role,
            approver_user_id: r.approver_user_id.map(|u| u.to_string()),
            require_all: r.require_all,
        }
    }
}

impl From<DecisionRow> for ApprovalDecision {
    fn from(r: DecisionRow) -> Self {
        ApprovalDecision {
            id: r.id.to_string(),
            request_id: r.request_id.to_string(),
            step_order: r.step_order,
            approver_user_id: r.approver_user_id.map(|u| u.to_string()),
            decision: r.decision,
            notes: r.notes,
            decided_at: r.decided_at,
        }
    }
}

const CHAIN_COLS: &str =
    "id, organization_id, name, entity_type, description, is_active, created_at, updated_at";
const REQUEST_COLS: &str = "id, organization_id, chain_id, entity_type, entity_id, \
     status::TEXT, current_step, requested_by, notes, completed_at, created_at, updated_at";

async fn fetch_steps(pool: &PgPool, chain_id: Uuid) -> Result<Vec<ApprovalChainStep>, DbError> {
    let rows: Vec<StepRow> = sqlx::query_as(
        "SELECT id, chain_id, step_order, required_role, approver_user_id, require_all \
         FROM approval_chain_steps WHERE chain_id = $1 ORDER BY step_order ASC",
    )
    .bind(chain_id)
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_err)?;
    Ok(rows.into_iter().map(ApprovalChainStep::from).collect())
}

async fn fetch_decisions(
    pool: &PgPool,
    request_id: Uuid,
) -> Result<Vec<ApprovalDecision>, DbError> {
    let rows: Vec<DecisionRow> = sqlx::query_as(
        "SELECT id, request_id, step_order, approver_user_id, decision, notes, decided_at \
         FROM approval_decisions WHERE request_id = $1 ORDER BY decided_at ASC",
    )
    .bind(request_id)
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_err)?;
    Ok(rows.into_iter().map(ApprovalDecision::from).collect())
}

fn to_chain(r: ChainRow, steps: Vec<ApprovalChainStep>) -> ApprovalChain {
    ApprovalChain {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        name: r.name,
        entity_type: r.entity_type,
        description: r.description,
        is_active: r.is_active,
        steps,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

fn to_request(r: RequestRow, decisions: Vec<ApprovalDecision>) -> ApprovalRequest {
    ApprovalRequest {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        chain_id: r.chain_id.to_string(),
        entity_type: r.entity_type,
        entity_id: r.entity_id.to_string(),
        status: r.status,
        current_step: r.current_step,
        requested_by: r.requested_by.map(|u| u.to_string()),
        notes: r.notes,
        completed_at: r.completed_at,
        decisions,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

pub struct ApprovalChainRepo;

impl ApprovalChainRepo {
    pub async fn create_chain(
        pool: &PgPool,
        org_id: &str,
        input: CreateApprovalChain,
    ) -> Result<ApprovalChain, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        if input.steps.is_empty() {
            return Err(DbError::Conflict(
                "approval chain must have at least one step".into(),
            ));
        }

        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO approval_chains \
             (id, organization_id, name, entity_type, description) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(&input.name)
        .bind(&input.entity_type)
        .bind(&input.description)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        for step in &input.steps {
            let approver_uuid = step
                .approver_user_id
                .as_deref()
                .map(parse_uuid)
                .transpose()?;
            sqlx::query(
                "INSERT INTO approval_chain_steps \
                 (chain_id, step_order, required_role, approver_user_id, require_all) \
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(id)
            .bind(step.step_order)
            .bind(&step.required_role)
            .bind(approver_uuid)
            .bind(step.require_all)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;
        }

        Self::get_chain(pool, org_id, &id.to_string()).await
    }

    pub async fn get_chain(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<ApprovalChain, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let c_uuid = parse_uuid(id)?;
        let row: ChainRow = sqlx::query_as(&format!(
            "SELECT {CHAIN_COLS} FROM approval_chains \
             WHERE id = $1 AND organization_id = $2"
        ))
        .bind(c_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        let steps = fetch_steps(pool, c_uuid).await?;
        Ok(to_chain(row, steps))
    }

    pub async fn list_chains(pool: &PgPool, org_id: &str) -> Result<Vec<ApprovalChain>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<ChainRow> = sqlx::query_as(&format!(
            "SELECT {CHAIN_COLS} FROM approval_chains \
             WHERE organization_id = $1 ORDER BY created_at DESC"
        ))
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        let mut chains = Vec::with_capacity(rows.len());
        for row in rows {
            let steps = fetch_steps(pool, row.id).await?;
            chains.push(to_chain(row, steps));
        }
        Ok(chains)
    }

    pub async fn submit_request(
        pool: &PgPool,
        org_id: &str,
        requester_id: &str,
        input: SubmitApprovalRequest,
    ) -> Result<ApprovalRequest, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let chain_uuid = parse_uuid(&input.chain_id)?;
        let entity_uuid = parse_uuid(&input.entity_id)?;
        let requester_uuid = parse_uuid(requester_id)?;

        // Verify chain belongs to org and is active.
        let chain: Option<(bool,)> = sqlx::query_as(
            "SELECT is_active FROM approval_chains WHERE id = $1 AND organization_id = $2",
        )
        .bind(chain_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;
        match chain {
            None => return Err(DbError::NotFound),
            Some((false,)) => return Err(DbError::Conflict("approval chain is inactive".into())),
            _ => {}
        }

        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO approval_requests \
             (id, organization_id, chain_id, entity_type, entity_id, requested_by, notes) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(chain_uuid)
        .bind(&input.entity_type)
        .bind(entity_uuid)
        .bind(requester_uuid)
        .bind(&input.notes)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_request(pool, org_id, &id.to_string()).await
    }

    pub async fn get_request(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<ApprovalRequest, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let r_uuid = parse_uuid(id)?;
        let row: RequestRow = sqlx::query_as(&format!(
            "SELECT {REQUEST_COLS} FROM approval_requests \
             WHERE id = $1 AND organization_id = $2"
        ))
        .bind(r_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        let decisions = fetch_decisions(pool, r_uuid).await?;
        Ok(to_request(row, decisions))
    }

    pub async fn list_requests(
        pool: &PgPool,
        org_id: &str,
        status: Option<&str>,
    ) -> Result<Vec<ApprovalRequest>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<RequestRow> = sqlx::query_as(&format!(
            "SELECT {REQUEST_COLS} FROM approval_requests \
             WHERE organization_id = $1 \
               AND ($2::text IS NULL OR status::TEXT = $2) \
             ORDER BY created_at DESC"
        ))
        .bind(org_uuid)
        .bind(status)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        let mut requests = Vec::with_capacity(rows.len());
        for row in rows {
            let decisions = fetch_decisions(pool, row.id).await?;
            requests.push(to_request(row, decisions));
        }
        Ok(requests)
    }

    pub async fn decide(
        pool: &PgPool,
        org_id: &str,
        request_id: &str,
        approver_id: &str,
        input: RecordApprovalDecision,
    ) -> Result<ApprovalRequest, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let r_uuid = parse_uuid(request_id)?;
        let approver_uuid = parse_uuid(approver_id)?;

        if !matches!(input.decision.as_str(), "approved" | "rejected") {
            return Err(DbError::Conflict(
                "decision must be 'approved' or 'rejected'".into(),
            ));
        }

        // Fetch request state.
        let row: Option<RequestRow> = sqlx::query_as(&format!(
            "SELECT {REQUEST_COLS} FROM approval_requests \
             WHERE id = $1 AND organization_id = $2"
        ))
        .bind(r_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        let req = row.ok_or(DbError::NotFound)?;
        if req.status != "pending" {
            return Err(DbError::Conflict(format!(
                "request is already '{}'",
                req.status
            )));
        }

        // Record the decision.
        sqlx::query(
            "INSERT INTO approval_decisions \
             (request_id, step_order, approver_user_id, decision, notes) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(r_uuid)
        .bind(req.current_step)
        .bind(approver_uuid)
        .bind(&input.decision)
        .bind(&input.notes)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        if input.decision == "rejected" {
            sqlx::query(
                "UPDATE approval_requests \
                 SET status = 'rejected', completed_at = NOW(), updated_at = NOW() \
                 WHERE id = $1",
            )
            .bind(r_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;
        } else {
            // Check if there's a next step in the chain.
            let next_step: Option<(i32,)> = sqlx::query_as(
                "SELECT step_order FROM approval_chain_steps \
                 WHERE chain_id = $1 AND step_order > $2 \
                 ORDER BY step_order ASC LIMIT 1",
            )
            .bind(req.chain_id)
            .bind(req.current_step)
            .fetch_optional(pool)
            .await
            .map_err(map_sqlx_err)?;

            if let Some((next,)) = next_step {
                sqlx::query(
                    "UPDATE approval_requests \
                     SET current_step = $2, updated_at = NOW() WHERE id = $1",
                )
                .bind(r_uuid)
                .bind(next)
                .execute(pool)
                .await
                .map_err(map_sqlx_err)?;
            } else {
                // All steps complete.
                sqlx::query(
                    "UPDATE approval_requests \
                     SET status = 'approved', completed_at = NOW(), updated_at = NOW() \
                     WHERE id = $1",
                )
                .bind(r_uuid)
                .execute(pool)
                .await
                .map_err(map_sqlx_err)?;
            }
        }

        Self::get_request(pool, org_id, request_id).await
    }
}
