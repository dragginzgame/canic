use super::super::executor::DeploymentExecutionPreflightError;
use super::super::{
    PromotionArtifactLevelV1, PromotionReadinessStatusV1, RoleArtifactSourceKindV1,
    SafetySeverityV1,
};
use thiserror::Error as ThisError;

///
/// PromotionArtifactSourceError
///
#[derive(Debug, ThisError)]
pub enum PromotionArtifactSourceError {
    #[error("promotion artifact source is missing required field: {field}")]
    MissingRequiredField { field: &'static str },
    #[error("promotion artifact source field {field} must be a lowercase sha256 hex digest")]
    InvalidSha256Digest { field: &'static str },
    #[error("promotion artifact source kind {kind:?} requires a digest pin")]
    MissingDigestPin { kind: RoleArtifactSourceKindV1 },
    #[error("promotion artifact source kind {kind:?} cannot carry previous receipt kind")]
    UnexpectedPreviousReceiptKind { kind: RoleArtifactSourceKindV1 },
    #[error(
        "promotion artifact source kind PreviousReceiptArtifact requires an eligible receipt kind"
    )]
    MissingPreviousReceiptKind,
    #[error(
        "promotion artifact source kind PreviousReceiptArtifact requires a source receipt lineage digest"
    )]
    MissingPreviousReceiptLineageDigest,
    #[error("promotion artifact source kind {kind:?} cannot carry source receipt lineage digest")]
    UnexpectedPreviousReceiptLineageDigest { kind: RoleArtifactSourceKindV1 },
}

///
/// PromotionReadinessError
///
#[derive(Debug, ThisError)]
pub enum PromotionReadinessError {
    #[error("promotion readiness schema mismatch: expected {expected}, found {found}")]
    SchemaVersionMismatch { expected: u32, found: u32 },
    #[error("promotion readiness is missing required field: {field}")]
    MissingRequiredField { field: &'static str },
    #[error("promotion readiness status {status:?} does not match blocker count {blocker_count}")]
    StatusBlockerMismatch {
        status: PromotionReadinessStatusV1,
        blocker_count: usize,
    },
    #[error("promotion readiness contains duplicate role: {role}")]
    DuplicateRole { role: String },
    #[error("promotion readiness role {role} has inconsistent restage state")]
    RestageStateMismatch { role: String },
    #[error("promotion readiness finding in {field} has severity {severity:?}")]
    FindingSeverityMismatch {
        field: &'static str,
        severity: SafetySeverityV1,
    },
    #[error("promotion readiness field {field} must be a lowercase sha256 hex digest")]
    InvalidSha256Digest { field: &'static str },
    #[error("promotion readiness field {field} is inconsistent")]
    LinkageMismatch { field: &'static str },
}

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
/// ArtifactPromotionPlanError
///
#[derive(Debug, ThisError)]
pub enum ArtifactPromotionPlanError {
    #[error("artifact promotion plan schema mismatch: expected {expected}, found {found}")]
    SchemaVersionMismatch { expected: u32, found: u32 },
    #[error("artifact promotion plan is missing required field: {field}")]
    MissingRequiredField { field: &'static str },
    #[error(
        "artifact promotion plan status {status:?} does not match blocker count {blocker_count}"
    )]
    StatusBlockerMismatch {
        status: PromotionReadinessStatusV1,
        blocker_count: usize,
    },
    #[error("artifact promotion plan field {field} is inconsistent")]
    LinkageMismatch { field: &'static str },
    #[error("artifact promotion plan field {field} must be a lowercase sha256 hex digest")]
    InvalidSha256Digest { field: &'static str },
    #[error("artifact promotion plan readiness is invalid: {0}")]
    Readiness(#[from] PromotionReadinessError),
    #[error("artifact promotion plan artifact identity report is invalid: {0}")]
    ArtifactIdentityReport(#[from] PromotionArtifactIdentityReportError),
    #[error("artifact promotion plan transform is invalid: {0}")]
    Transform(#[from] PromotionPlanTransformError),
    #[error("artifact promotion plan target execution lineage is invalid: {0}")]
    TargetExecutionLineage(#[from] PromotionTargetExecutionLineageError),
    #[error(
        "artifact promotion plan requires target execution lineage for deployment check validation"
    )]
    MissingTargetExecutionLineage,
    #[error("artifact promotion plan target deployment check is invalid: {0}")]
    TargetCheck(#[source] DeploymentExecutionPreflightError),
}

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
