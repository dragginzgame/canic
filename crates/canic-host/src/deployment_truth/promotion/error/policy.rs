use crate::deployment_truth::{
    PromotionArtifactLevelV1, PromotionReadinessStatusV1, SafetySeverityV1,
};
use thiserror::Error as ThisError;

///
/// PromotionPolicyCheckError
///
#[derive(Debug, ThisError)]
pub enum PromotionPolicyCheckError {
    #[error("promotion policy check schema mismatch: expected {expected}, found {found}")]
    SchemaVersionMismatch { expected: u32, found: u32 },
    #[error("promotion policy check is missing required field: {field}")]
    MissingRequiredField { field: &'static str },
    #[error(
        "promotion policy check status {status:?} does not match blocker count {blocker_count}"
    )]
    StatusBlockerMismatch {
        status: PromotionReadinessStatusV1,
        blocker_count: usize,
    },
    #[error("promotion policy check contains duplicate role: {role}")]
    DuplicateRole { role: String },
    #[error("promotion policy for role {role} has duplicate allowed level {level:?}")]
    DuplicateAllowedLevel {
        role: String,
        level: PromotionArtifactLevelV1,
    },
    #[error("promotion policy for role {role} has no allowed promotion levels")]
    EmptyAllowedLevels { role: String },
    #[error("promotion policy decision for role {role} has inconsistent field {field}")]
    DecisionMismatch { role: String, field: &'static str },
    #[error("promotion policy check field {field} is inconsistent")]
    LinkageMismatch { field: &'static str },
    #[error("promotion policy check field {field} must be a lowercase sha256 hex digest")]
    InvalidSha256Digest { field: &'static str },
    #[error("promotion policy check blocker has severity {severity:?}")]
    BlockerSeverityMismatch { severity: SafetySeverityV1 },
}
