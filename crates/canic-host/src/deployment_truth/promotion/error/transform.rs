use super::materialization::PromotionMaterializationIdentityError;
use super::readiness::PromotionReadinessError;
use crate::deployment_truth::executor::DeploymentExecutionPreflightError;
use thiserror::Error as ThisError;

///
/// PromotionPlanTransformError
///
#[derive(Debug, ThisError)]
pub enum PromotionPlanTransformError {
    #[error("promotion plan transform schema mismatch: expected {expected}, found {found}")]
    SchemaVersionMismatch { expected: u32, found: u32 },
    #[error("promotion plan transform is missing required field: {field}")]
    MissingRequiredField { field: &'static str },
    #[error("promotion readiness validation failed: {0}")]
    Readiness(#[from] PromotionReadinessError),
    #[error("promotion readiness is blocked with {blocker_count} blocker(s)")]
    ReadinessBlocked { blocker_count: usize },
    #[error("promotion target plan is missing role: {role}")]
    TargetRoleMissing { role: String },
    #[error("promotion transform contains duplicate source/build materialization for role: {role}")]
    DuplicateMaterializationRole { role: String },
    #[error(
        "promotion transform is missing source/build materialization evidence for role: {role}"
    )]
    MaterializationRoleMissing { role: String },
    #[error(
        "promotion transform contains unexpected source/build materialization for role: {role}"
    )]
    UnexpectedMaterializationRole { role: String },
    #[error("promotion materialization evidence is invalid: {0}")]
    Materialization(#[from] PromotionMaterializationIdentityError),
    #[error("promotion transform contains duplicate role: {role}")]
    DuplicateRole { role: String },
    #[error("promotion transform promoted plan id mismatch: expected {expected}, found {found}")]
    PromotedPlanIdMismatch { expected: String, found: String },
    #[error("promotion transform role {role} is missing from promoted plan")]
    PromotedRoleMissing { role: String },
    #[error("promotion transform role {role} has inconsistent field {field}")]
    RoleStateMismatch { role: String, field: &'static str },
}

///
/// PromotionPlanTransformEvidenceError
///
#[derive(Debug, ThisError)]
pub enum PromotionPlanTransformEvidenceError {
    #[error(
        "promotion plan transform evidence schema mismatch: expected {expected}, found {found}"
    )]
    SchemaVersionMismatch { expected: u32, found: u32 },
    #[error("promotion plan transform evidence is missing required field: {field}")]
    MissingRequiredField { field: &'static str },
    #[error(
        "promotion plan transform evidence field {field} must be a lowercase sha256 hex digest"
    )]
    InvalidSha256Digest { field: &'static str },
    #[error("promotion plan transform evidence field {field} is inconsistent")]
    LinkageMismatch { field: &'static str },
    #[error("promotion plan transform evidence has invalid transform: {0}")]
    Transform(#[from] PromotionPlanTransformError),
}

///
/// PromotionTargetExecutionLineageError
///
#[derive(Debug, ThisError)]
pub enum PromotionTargetExecutionLineageError {
    #[error(
        "promotion target execution lineage schema mismatch: expected {expected}, found {found}"
    )]
    SchemaVersionMismatch { expected: u32, found: u32 },
    #[error("promotion target execution lineage is missing required field: {field}")]
    MissingRequiredField { field: &'static str },
    #[error(
        "promotion target execution lineage field {field} must be a lowercase sha256 hex digest"
    )]
    InvalidSha256Digest { field: &'static str },
    #[error("promotion target execution lineage has invalid transform: {0}")]
    Transform(#[from] PromotionPlanTransformError),
    #[error("promotion target execution lineage has invalid execution preflight: {0}")]
    Preflight(#[from] DeploymentExecutionPreflightError),
    #[error("promotion target execution lineage field {field} is inconsistent")]
    LinkageMismatch { field: &'static str },
    #[error("promotion target execution lineage must not claim execution occurred")]
    ExecutionAttempted,
}
