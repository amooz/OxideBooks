use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalChainStep {
    pub id: String,
    pub chain_id: String,
    pub step_order: i32,
    pub required_role: String,
    pub approver_user_id: Option<String>,
    pub require_all: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalChain {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub entity_type: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub steps: Vec<ApprovalChainStep>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalDecision {
    pub id: String,
    pub request_id: String,
    pub step_order: i32,
    pub approver_user_id: Option<String>,
    pub decision: String,
    pub notes: Option<String>,
    pub decided_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: String,
    pub organization_id: String,
    pub chain_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub status: String,
    pub current_step: i32,
    pub requested_by: Option<String>,
    pub notes: Option<String>,
    pub completed_at: Option<OffsetDateTime>,
    pub decisions: Vec<ApprovalDecision>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateApprovalChainStep {
    pub step_order: i32,
    pub required_role: String,
    pub approver_user_id: Option<String>,
    #[serde(default)]
    pub require_all: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateApprovalChain {
    pub name: String,
    pub entity_type: String,
    pub description: Option<String>,
    pub steps: Vec<CreateApprovalChainStep>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubmitApprovalRequest {
    pub chain_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecordApprovalDecision {
    pub decision: String,
    pub notes: Option<String>,
}
