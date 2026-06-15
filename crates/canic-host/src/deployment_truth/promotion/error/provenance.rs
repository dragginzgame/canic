use super::artifact_plan::ArtifactPromotionPlanError;
use super::materialization::PromotionMaterializationIdentityReportError;
use super::wasm_store::{
    PromotionWasmStoreCatalogVerificationError, PromotionWasmStoreIdentityReportError,
};
use crate::deployment_truth::{PromotionReadinessStatusV1, SafetySeverityV1};
use thiserror::Error as ThisError;

///
/// ArtifactPromotionProvenanceReportError
///
#[derive(Debug, ThisError)]
pub enum ArtifactPromotionProvenanceReportError {
    #[error(
        "artifact promotion provenance report schema mismatch: expected {expected}, found {found}"
    )]
    SchemaVersionMismatch { expected: u32, found: u32 },
    #[error("artifact promotion provenance report is missing required field: {field}")]
    MissingRequiredField { field: &'static str },
    #[error(
        "artifact promotion provenance report status {status:?} does not match blocker count {blocker_count}"
    )]
    StatusBlockerMismatch {
        status: PromotionReadinessStatusV1,
        blocker_count: usize,
    },
    #[error("artifact promotion provenance report field {field} is inconsistent")]
    LinkageMismatch { field: &'static str },
    #[error("artifact promotion provenance report contains duplicate role: {role}")]
    DuplicateRole { role: String },
    #[error("artifact promotion provenance report blockers are stale")]
    BlockerMismatch,
    #[error("artifact promotion provenance report blocker has severity {severity:?}")]
    BlockerSeverityMismatch { severity: SafetySeverityV1 },
    #[error(
        "artifact promotion provenance report field {field} must be a lowercase sha256 hex digest"
    )]
    InvalidSha256Digest { field: &'static str },
    #[error("artifact promotion provenance report has invalid artifact promotion plan: {0}")]
    Plan(#[from] ArtifactPromotionPlanError),
    #[error("artifact promotion provenance report has invalid wasm-store identity report: {0}")]
    WasmStoreIdentity(#[from] PromotionWasmStoreIdentityReportError),
    #[error(
        "artifact promotion provenance report has invalid wasm-store catalog verification: {0}"
    )]
    WasmStoreCatalog(#[from] PromotionWasmStoreCatalogVerificationError),
    #[error(
        "artifact promotion provenance report has invalid materialization identity report: {0}"
    )]
    MaterializationIdentity(#[from] PromotionMaterializationIdentityReportError),
}

///
/// ArtifactPromotionExecutionReceiptError
///
#[derive(Debug, ThisError)]
pub enum ArtifactPromotionExecutionReceiptError {
    #[error(
        "artifact promotion execution receipt schema mismatch: expected {expected}, found {found}"
    )]
    SchemaVersionMismatch { expected: u32, found: u32 },
    #[error("artifact promotion execution receipt is missing required field: {field}")]
    MissingRequiredField { field: &'static str },
    #[error("artifact promotion execution receipt field {field} is inconsistent")]
    LinkageMismatch { field: &'static str },
    #[error("artifact promotion execution receipt contains unknown deployment role: {role}")]
    UnknownDeploymentRole { role: String },
    #[error("artifact promotion execution receipt is missing deployment role: {role}")]
    MissingDeploymentRole { role: String },
    #[error("artifact promotion execution receipt provenance status {status:?} is not ready")]
    ProvenanceNotReady { status: PromotionReadinessStatusV1 },
    #[error("artifact promotion execution receipt has invalid provenance report: {0}")]
    Provenance(#[from] ArtifactPromotionProvenanceReportError),
}
