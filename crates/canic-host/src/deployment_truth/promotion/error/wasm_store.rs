use crate::deployment_truth::{PromotionReadinessStatusV1, SafetySeverityV1};
use thiserror::Error as ThisError;

///
/// PromotionWasmStoreIdentityReportError
///
#[derive(Debug, ThisError)]
pub enum PromotionWasmStoreIdentityReportError {
    #[error(
        "promotion wasm-store identity report schema mismatch: expected {expected}, found {found}"
    )]
    SchemaVersionMismatch { expected: u32, found: u32 },
    #[error("promotion wasm-store identity report is missing required field: {field}")]
    MissingRequiredField { field: &'static str },
    #[error(
        "promotion wasm-store identity report status {status:?} does not match blocker count {blocker_count}"
    )]
    StatusBlockerMismatch {
        status: PromotionReadinessStatusV1,
        blocker_count: usize,
    },
    #[error("promotion wasm-store identity report contains duplicate role: {role}")]
    DuplicateRole { role: String },
    #[error(
        "promotion wasm-store identity report staging receipt schema mismatch for role {role}: expected {expected}, found {found}"
    )]
    StagingReceiptSchemaVersionMismatch {
        role: String,
        expected: u32,
        found: u32,
    },
    #[error("promotion wasm-store identity report blockers are stale")]
    BlockerMismatch,
    #[error("promotion wasm-store identity report field {field} is inconsistent")]
    LinkageMismatch { field: &'static str },
    #[error(
        "promotion wasm-store identity report field {field} must be a lowercase sha256 hex digest"
    )]
    InvalidSha256Digest { field: &'static str },
    #[error("promotion wasm-store identity report blocker has severity {severity:?}")]
    BlockerSeverityMismatch { severity: SafetySeverityV1 },
}

///
/// PromotionWasmStoreCatalogVerificationError
///
#[derive(Debug, ThisError)]
pub enum PromotionWasmStoreCatalogVerificationError {
    #[error(
        "promotion wasm-store catalog verification schema mismatch: expected {expected}, found {found}"
    )]
    SchemaVersionMismatch { expected: u32, found: u32 },
    #[error("promotion wasm-store catalog verification is missing required field: {field}")]
    MissingRequiredField { field: &'static str },
    #[error(
        "promotion wasm-store catalog verification status {status:?} does not match blocker count {blocker_count}"
    )]
    StatusBlockerMismatch {
        status: PromotionReadinessStatusV1,
        blocker_count: usize,
    },
    #[error("promotion wasm-store catalog verification contains duplicate role: {role}")]
    DuplicateRole { role: String },
    #[error("promotion wasm-store catalog verification contains duplicate locator: {locator}")]
    DuplicateLocator { locator: String },
    #[error("promotion wasm-store catalog verification role {role} has inconsistent field {field}")]
    RoleMismatch { role: String, field: &'static str },
    #[error("promotion wasm-store catalog verification field {field} is inconsistent")]
    LinkageMismatch { field: &'static str },
    #[error(
        "promotion wasm-store catalog verification field {field} must be a lowercase sha256 hex digest"
    )]
    InvalidSha256Digest { field: &'static str },
    #[error("promotion wasm-store catalog verification blockers are stale")]
    BlockerMismatch,
    #[error("promotion wasm-store catalog verification blocker has severity {severity:?}")]
    BlockerSeverityMismatch { severity: SafetySeverityV1 },
    #[error(
        "promotion wasm-store catalog verification has invalid wasm-store identity report: {0}"
    )]
    WasmStoreIdentity(#[from] PromotionWasmStoreIdentityReportError),
}
