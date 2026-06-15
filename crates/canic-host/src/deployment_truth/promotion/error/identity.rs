use crate::deployment_truth::{PromotionReadinessStatusV1, SafetySeverityV1};
use thiserror::Error as ThisError;

///
/// PromotionArtifactIdentityReportError
///
#[derive(Debug, ThisError)]
pub enum PromotionArtifactIdentityReportError {
    #[error(
        "promotion artifact identity report schema mismatch: expected {expected}, found {found}"
    )]
    SchemaVersionMismatch { expected: u32, found: u32 },
    #[error("promotion artifact identity report is missing required field: {field}")]
    MissingRequiredField { field: &'static str },
    #[error(
        "promotion artifact identity report status {status:?} does not match blocker count {blocker_count}"
    )]
    StatusBlockerMismatch {
        status: PromotionReadinessStatusV1,
        blocker_count: usize,
    },
    #[error("promotion artifact identity report contains duplicate role: {role}")]
    DuplicateRole { role: String },
    #[error("promotion artifact identity report contains duplicate identity group: {identity_key}")]
    DuplicateIdentityGroup { identity_key: String },
    #[error("promotion artifact identity report identity group {identity_key} has no roles")]
    EmptyIdentityGroup { identity_key: String },
    #[error("promotion artifact identity report identity group contains unknown role: {role}")]
    UnknownGroupedRole { role: String },
    #[error("promotion artifact identity report groups role {role} more than once")]
    DuplicateGroupedRole { role: String },
    #[error("promotion artifact identity report does not group role: {role}")]
    MissingGroupedRole { role: String },
    #[error(
        "promotion artifact identity report role {role} belongs to identity group {expected}, found {found}"
    )]
    IdentityGroupRoleMismatch {
        role: String,
        expected: String,
        found: String,
    },
    #[error(
        "promotion artifact identity report identity group key mismatch: expected {expected}, found {found}"
    )]
    IdentityGroupKeyMismatch { expected: String, found: String },
    #[error("promotion artifact identity report summary field {field} is stale")]
    SummaryMismatch { field: &'static str },
    #[error("promotion artifact identity report field {field} is inconsistent")]
    LinkageMismatch { field: &'static str },
    #[error(
        "promotion artifact identity report field {field} must be a lowercase sha256 hex digest"
    )]
    InvalidSha256Digest { field: &'static str },
    #[error("promotion artifact identity report blocker has severity {severity:?}")]
    BlockerSeverityMismatch { severity: SafetySeverityV1 },
}
