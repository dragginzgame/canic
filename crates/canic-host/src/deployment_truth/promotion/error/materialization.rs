use crate::deployment_truth::{PromotionReadinessStatusV1, SafetySeverityV1};
use thiserror::Error as ThisError;

///
/// PromotionMaterializationIdentityError
///
#[derive(Debug, ThisError)]
pub enum PromotionMaterializationIdentityError {
    #[error(
        "promotion materialization identity schema mismatch: expected {expected}, found {found}"
    )]
    SchemaVersionMismatch { expected: u32, found: u32 },
    #[error("promotion materialization identity is missing required field: {field}")]
    MissingRequiredField { field: &'static str },
    #[error(
        "promotion materialization identity field {field} must be a lowercase sha256 hex digest"
    )]
    InvalidSha256Digest { field: &'static str },
    #[error("promotion materialization identity field {field} is inconsistent")]
    LinkageMismatch { field: &'static str },
    #[error(
        "promotion materialization identity digest mismatch for {field}: expected {expected}, found {found}"
    )]
    DigestMismatch {
        field: &'static str,
        expected: String,
        found: String,
    },
}

///
/// PromotionMaterializationIdentityReportError
///
#[derive(Debug, ThisError)]
pub enum PromotionMaterializationIdentityReportError {
    #[error(
        "promotion materialization identity report schema mismatch: expected {expected}, found {found}"
    )]
    SchemaVersionMismatch { expected: u32, found: u32 },
    #[error("promotion materialization identity report is missing required field: {field}")]
    MissingRequiredField { field: &'static str },
    #[error(
        "promotion materialization identity report status {status:?} does not match blocker count {blocker_count}"
    )]
    StatusBlockerMismatch {
        status: PromotionReadinessStatusV1,
        blocker_count: usize,
    },
    #[error("promotion materialization identity report contains duplicate role: {role}")]
    DuplicateRole { role: String },
    #[error("promotion materialization identity report contains duplicate evidence: {evidence_id}")]
    DuplicateEvidence { evidence_id: String },
    #[error(
        "promotion materialization identity report contains duplicate output group: {output_identity_key}"
    )]
    DuplicateOutputGroup { output_identity_key: String },
    #[error(
        "promotion materialization identity report output group {output_identity_key} has no roles"
    )]
    EmptyOutputGroup { output_identity_key: String },
    #[error("promotion materialization identity report output group contains unknown role: {role}")]
    UnknownGroupedRole { role: String },
    #[error("promotion materialization identity report groups role {role} more than once")]
    DuplicateGroupedRole { role: String },
    #[error("promotion materialization identity report does not group role: {role}")]
    MissingGroupedRole { role: String },
    #[error(
        "promotion materialization identity report role {role} belongs to output group {expected}, found {found}"
    )]
    OutputGroupRoleMismatch {
        role: String,
        expected: String,
        found: String,
    },
    #[error(
        "promotion materialization identity report output group key mismatch: expected {expected}, found {found}"
    )]
    OutputGroupKeyMismatch { expected: String, found: String },
    #[error("promotion materialization identity report field {field} is inconsistent")]
    LinkageMismatch { field: &'static str },
    #[error(
        "promotion materialization identity report field {field} must be a lowercase sha256 hex digest"
    )]
    InvalidSha256Digest { field: &'static str },
    #[error("promotion materialization identity report blockers are stale")]
    BlockerMismatch,
    #[error("promotion materialization identity report blocker has severity {severity:?}")]
    BlockerSeverityMismatch { severity: SafetySeverityV1 },
    #[error("promotion materialization identity report has invalid materialization evidence: {0}")]
    Materialization(#[from] PromotionMaterializationIdentityError),
}
