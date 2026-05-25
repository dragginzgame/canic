use super::executor::{
    DeploymentExecutionPreflightError, validate_deployment_execution_preflight,
    validate_deployment_execution_preflight_for_check,
};
use super::{
    ArtifactPromotionExecutionReceiptV1, ArtifactPromotionPlanV1,
    ArtifactPromotionProvenanceReportV1, ArtifactSourceV1, ArtifactTransportV1,
    BuildMaterializationEvidenceV1, BuildMaterializationInputV1, BuildMaterializationResultV1,
    BuildRecipeIdentityV1, DEPLOYMENT_TRUTH_SCHEMA_VERSION, DeploymentCheckV1,
    DeploymentExecutionPreflightStatusV1, DeploymentExecutionPreflightV1, DeploymentPlanV1,
    DeploymentReceiptV1, ObservationStatusV1, PromotionArtifactIdentityGroupV1,
    PromotionArtifactIdentityKindV1, PromotionArtifactIdentityReportV1,
    PromotionArtifactIdentitySummaryV1, PromotionArtifactLevelV1,
    PromotionMaterializationIdentityReportV1, PromotionMaterializationOutputGroupV1,
    PromotionPlanTransformEvidenceV1, PromotionPlanTransformV1, PromotionPolicyCheckV1,
    PromotionPolicyClaimV1, PromotionPolicyRequirementV1, PromotionReadinessStatusV1,
    PromotionReadinessV1, PromotionTargetExecutionLineageV1, PromotionWasmStoreIdentityReportV1,
    RoleArtifactSourceKindV1, RoleArtifactSourceV1, RoleArtifactV1,
    RolePromotionArtifactIdentityV1, RolePromotionExecutionReceiptV1, RolePromotionInputV1,
    RolePromotionMaterializationIdentityV1, RolePromotionMaterializationLinkV1,
    RolePromotionPlanTransformV1, RolePromotionPolicyDecisionV1, RolePromotionPolicyV1,
    RolePromotionProvenanceV1, RolePromotionReadinessV1, RolePromotionWasmStoreIdentityV1,
    SafetyFindingV1, SafetySeverityV1, StagingReceiptV1, stable_json_sha256_hex,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
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
    #[error("artifact promotion provenance report has invalid artifact promotion plan: {0}")]
    Plan(#[from] ArtifactPromotionPlanError),
    #[error("artifact promotion provenance report has invalid wasm-store identity report: {0}")]
    WasmStoreIdentity(#[from] PromotionWasmStoreIdentityReportError),
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
    #[error("promotion wasm-store identity report blocker has severity {severity:?}")]
    BlockerSeverityMismatch { severity: SafetySeverityV1 },
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
    #[error("promotion policy check blocker has severity {severity:?}")]
    BlockerSeverityMismatch { severity: SafetySeverityV1 },
}

///
/// PromotionReadinessRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionReadinessRequest {
    pub readiness_id: String,
    pub target_plan: DeploymentPlanV1,
    pub inputs: Vec<RolePromotionInputV1>,
}

///
/// PromotionReadinessWithPolicyRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionReadinessWithPolicyRequest {
    pub readiness_id: String,
    pub target_plan: DeploymentPlanV1,
    pub inputs: Vec<RolePromotionInputV1>,
    pub policies: Vec<RolePromotionPolicyV1>,
}

///
/// PromotionPlanTransformRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionPlanTransformRequest {
    pub promoted_plan_id: String,
    pub target_plan: DeploymentPlanV1,
    pub inputs: Vec<RolePromotionInputV1>,
}

///
/// PromotionPlanTransformWithMaterializationRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionPlanTransformWithMaterializationRequest {
    pub promoted_plan_id: String,
    pub target_plan: DeploymentPlanV1,
    pub inputs: Vec<RolePromotionInputV1>,
    pub materialization_evidence: Vec<BuildMaterializationEvidenceV1>,
}

///
/// PromotionPlanTransformEvidenceRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionPlanTransformEvidenceRequest {
    pub evidence_id: String,
    pub generated_at: String,
    pub transform: PromotionPlanTransformV1,
}

///
/// ArtifactPromotionPlanRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactPromotionPlanRequest {
    pub plan_id: String,
    pub generated_at: String,
    pub readiness: PromotionReadinessV1,
    pub artifact_identity_report: PromotionArtifactIdentityReportV1,
    pub transform: PromotionPlanTransformV1,
    pub target_execution_lineage: Option<PromotionTargetExecutionLineageV1>,
}

///
/// ArtifactPromotionProvenanceReportRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactPromotionProvenanceReportRequest {
    pub report_id: String,
    pub artifact_promotion_plan: ArtifactPromotionPlanV1,
    pub wasm_store_identity_report: Option<PromotionWasmStoreIdentityReportV1>,
    pub materialization_identity_report: Option<PromotionMaterializationIdentityReportV1>,
}

///
/// ArtifactPromotionExecutionReceiptRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactPromotionExecutionReceiptRequest {
    pub receipt_id: String,
    pub provenance_report: ArtifactPromotionProvenanceReportV1,
    pub deployment_receipt: DeploymentReceiptV1,
}

///
/// PromotionTargetExecutionLineageRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionTargetExecutionLineageRequest {
    pub lineage_id: String,
    pub generated_at: String,
    pub transform: PromotionPlanTransformV1,
    pub execution_preflight: DeploymentExecutionPreflightV1,
}

///
/// PromotionArtifactIdentityReportRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionArtifactIdentityReportRequest {
    pub report_id: String,
    pub inputs: Vec<RolePromotionInputV1>,
}

///
/// PromotionWasmStoreIdentityReportRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionWasmStoreIdentityReportRequest {
    pub report_id: String,
    pub staging_receipts: Vec<StagingReceiptV1>,
}

///
/// BuildMaterializationEvidenceRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildMaterializationEvidenceRequest {
    pub evidence_id: String,
    pub recipe: BuildRecipeIdentityV1,
    pub materialization_input: BuildMaterializationInputV1,
    pub materialization_result: BuildMaterializationResultV1,
}

///
/// PromotionMaterializationIdentityReportRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionMaterializationIdentityReportRequest {
    pub report_id: String,
    pub evidence: Vec<BuildMaterializationEvidenceV1>,
}

///
/// PromotionPolicyCheckRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionPolicyCheckRequest {
    pub check_id: String,
    pub inputs: Vec<RolePromotionInputV1>,
    pub policies: Vec<RolePromotionPolicyV1>,
}

#[derive(Serialize)]
struct PromotionPlanLineageInput<'a> {
    target_plan_id: &'a str,
    promoted_plan_id: &'a str,
    promoted_plan: &'a DeploymentPlanV1,
    roles: &'a [RolePromotionPlanTransformV1],
}

#[derive(Serialize)]
struct PromotionTargetExecutionLineageInput<'a> {
    promotion_plan_lineage_digest: &'a str,
    promoted_plan_id: &'a str,
    preflight_plan_id: &'a str,
    preflight_safety_report_id: &'a str,
    preflight_authority_plan_id: &'a str,
    preflight_backend: &'a super::DeploymentExecutorBackendV1,
    preflight_status: DeploymentExecutionPreflightStatusV1,
    planned_phases: &'a [String],
    required_capabilities: &'a [super::DeploymentExecutorCapabilityV1],
    missing_capabilities: &'a [super::DeploymentExecutorCapabilityV1],
    execution_attempted: bool,
}

pub fn promoted_deployment_plan_from_inputs(
    request: &PromotionPlanTransformRequest,
) -> Result<DeploymentPlanV1, PromotionPlanTransformError> {
    Ok(promoted_deployment_plan_transform_from_inputs(request)?.promoted_plan)
}

pub fn promoted_deployment_plan_transform_from_inputs(
    request: &PromotionPlanTransformRequest,
) -> Result<PromotionPlanTransformV1, PromotionPlanTransformError> {
    ensure_transform_field("promoted_plan_id", &request.promoted_plan_id)?;
    let readiness = promotion_readiness_from_inputs(
        &request.promoted_plan_id,
        &request.target_plan,
        &request.inputs,
    );
    validate_promotion_readiness(&readiness)?;
    if readiness.status == PromotionReadinessStatusV1::Blocked {
        return Err(PromotionPlanTransformError::ReadinessBlocked {
            blocker_count: readiness.blockers.len(),
        });
    }

    let mut promoted_plan = request.target_plan.clone();
    promoted_plan.plan_id.clone_from(&request.promoted_plan_id);
    for input in &request.inputs {
        let Some(role_artifact) = promoted_plan
            .role_artifacts
            .iter_mut()
            .find(|artifact| artifact.role == input.role)
        else {
            return Err(PromotionPlanTransformError::TargetRoleMissing {
                role: input.role.clone(),
            });
        };
        apply_promotion_input_to_role_artifact(role_artifact, input);
    }
    let transform =
        promotion_plan_transform_from_parts(&request.target_plan, promoted_plan, &request.inputs);
    validate_promotion_plan_transform(&transform)?;
    Ok(transform)
}

pub fn promoted_deployment_plan_transform_from_inputs_with_materialization(
    request: &PromotionPlanTransformWithMaterializationRequest,
) -> Result<PromotionPlanTransformV1, PromotionPlanTransformError> {
    let base_request = PromotionPlanTransformRequest {
        promoted_plan_id: request.promoted_plan_id.clone(),
        target_plan: request.target_plan.clone(),
        inputs: request.inputs.clone(),
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&base_request)?;
    attach_source_build_materialization(
        &mut transform,
        &request.inputs,
        &request.materialization_evidence,
    )?;
    refresh_promotion_plan_lineage_digest(&mut transform);
    validate_promotion_plan_transform(&transform)?;
    Ok(transform)
}

pub fn check_promotion_readiness(
    request: &PromotionReadinessRequest,
) -> Result<PromotionReadinessV1, PromotionReadinessError> {
    ensure_readiness_field("readiness_id", &request.readiness_id)?;
    let readiness = promotion_readiness_from_inputs(
        &request.readiness_id,
        &request.target_plan,
        &request.inputs,
    );
    validate_promotion_readiness(&readiness)?;
    Ok(readiness)
}

pub fn check_promotion_readiness_with_policy(
    request: &PromotionReadinessWithPolicyRequest,
) -> Result<PromotionReadinessV1, PromotionReadinessError> {
    ensure_readiness_field("readiness_id", &request.readiness_id)?;
    let readiness = promotion_readiness_from_inputs_with_policy(
        &request.readiness_id,
        &request.target_plan,
        &request.inputs,
        &request.policies,
    );
    validate_promotion_readiness(&readiness)?;
    Ok(readiness)
}

pub fn check_promotion_policy(
    request: PromotionPolicyCheckRequest,
) -> Result<PromotionPolicyCheckV1, PromotionPolicyCheckError> {
    ensure_policy_field("check_id", &request.check_id)?;
    let check =
        promotion_policy_check_from_inputs(&request.check_id, &request.inputs, &request.policies);
    validate_promotion_policy_check(&check)?;
    Ok(check)
}

#[must_use]
pub fn promotion_policy_check_from_inputs(
    check_id: impl Into<String>,
    inputs: &[RolePromotionInputV1],
    policies: &[RolePromotionPolicyV1],
) -> PromotionPolicyCheckV1 {
    let mut roles = Vec::with_capacity(inputs.len());
    let mut blockers = Vec::new();
    let mut seen_policy_roles = BTreeSet::new();
    for policy in policies {
        if !seen_policy_roles.insert(policy.role.as_str()) {
            blockers.push(promotion_finding(
                "promotion_policy_duplicate",
                format!("multiple promotion policies exist for role {}", policy.role),
                SafetySeverityV1::HardFailure,
                &policy.role,
            ));
        }
        if let Err(err) = validate_role_promotion_policy(policy) {
            blockers.push(promotion_finding(
                "promotion_policy_invalid",
                err.to_string(),
                SafetySeverityV1::HardFailure,
                &policy.role,
            ));
        }
    }
    for input in inputs {
        let matching_policies = policies
            .iter()
            .filter(|policy| policy.role == input.role)
            .collect::<Vec<_>>();
        match matching_policies.as_slice() {
            [] => {
                blockers.push(promotion_finding(
                    "promotion_policy_missing",
                    format!("no promotion policy exists for role {}", input.role),
                    SafetySeverityV1::HardFailure,
                    &input.role,
                ));
            }
            [policy] => {
                let decision = role_promotion_policy_decision(input, policy);
                collect_policy_findings(&decision, &mut blockers);
                roles.push(decision);
            }
            _ => blockers.push(promotion_finding(
                "promotion_policy_duplicate",
                format!("multiple promotion policies exist for role {}", input.role),
                SafetySeverityV1::HardFailure,
                &input.role,
            )),
        }
    }

    PromotionPolicyCheckV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        check_id: check_id.into(),
        status: if blockers.is_empty() {
            PromotionReadinessStatusV1::Ready
        } else {
            PromotionReadinessStatusV1::Blocked
        },
        roles,
        blockers,
    }
}

pub fn validate_promotion_policy_check(
    check: &PromotionPolicyCheckV1,
) -> Result<(), PromotionPolicyCheckError> {
    if check.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(PromotionPolicyCheckError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            found: check.schema_version,
        });
    }
    ensure_policy_field("check_id", &check.check_id)?;
    ensure_policy_status_matches_blockers(check)?;
    ensure_unique_policy_decision_roles(&check.roles)?;
    for role in &check.roles {
        validate_role_promotion_policy_decision(role)?;
    }
    validate_policy_blockers(&check.blockers)?;
    Ok(())
}

pub fn promotion_artifact_identity_report_from_inputs(
    request: PromotionArtifactIdentityReportRequest,
) -> Result<PromotionArtifactIdentityReportV1, PromotionArtifactIdentityReportError> {
    ensure_identity_report_field("report_id", &request.report_id)?;
    let report = promotion_artifact_identity_report(&request.report_id, &request.inputs);
    validate_promotion_artifact_identity_report(&report)?;
    Ok(report)
}

pub fn promotion_wasm_store_identity_report_from_staging(
    request: PromotionWasmStoreIdentityReportRequest,
) -> Result<PromotionWasmStoreIdentityReportV1, PromotionWasmStoreIdentityReportError> {
    ensure_wasm_store_identity_report_field("report_id", &request.report_id)?;
    ensure_wasm_store_identity_staging_receipts(&request.staging_receipts)?;
    let report =
        promotion_wasm_store_identity_report(&request.report_id, &request.staging_receipts);
    validate_promotion_wasm_store_identity_report(&report)?;
    Ok(report)
}

#[must_use]
pub fn promotion_artifact_identity_report(
    report_id: impl Into<String>,
    inputs: &[RolePromotionInputV1],
) -> PromotionArtifactIdentityReportV1 {
    let mut roles = Vec::with_capacity(inputs.len());
    let mut blockers = Vec::new();
    for input in inputs {
        if let Err(err) = validate_role_artifact_source(&input.source) {
            blockers.push(promotion_finding(
                "promotion_artifact_source_invalid",
                err.to_string(),
                SafetySeverityV1::HardFailure,
                &input.role,
            ));
        }
        if input.role != input.source.role {
            blockers.push(promotion_finding(
                "promotion_source_role_mismatch",
                format!(
                    "promotion input role {} does not match artifact source role {}",
                    input.role, input.source.role
                ),
                SafetySeverityV1::HardFailure,
                &input.role,
            ));
        }
        roles.push(role_promotion_artifact_identity(input));
    }
    let identity_groups = promotion_artifact_identity_groups(&roles);
    let summary = promotion_artifact_identity_summary(&roles, &identity_groups);

    PromotionArtifactIdentityReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: report_id.into(),
        status: if blockers.is_empty() {
            PromotionReadinessStatusV1::Ready
        } else {
            PromotionReadinessStatusV1::Blocked
        },
        summary,
        identity_groups,
        roles,
        blockers,
    }
}

#[must_use]
pub fn promotion_wasm_store_identity_report(
    report_id: impl Into<String>,
    staging_receipts: &[StagingReceiptV1],
) -> PromotionWasmStoreIdentityReportV1 {
    let roles = staging_receipts
        .iter()
        .map(role_wasm_store_identity_from_staging)
        .collect::<Vec<_>>();
    let blockers = wasm_store_identity_blockers(&roles);
    PromotionWasmStoreIdentityReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: report_id.into(),
        status: if blockers.is_empty() {
            PromotionReadinessStatusV1::Ready
        } else {
            PromotionReadinessStatusV1::Blocked
        },
        roles,
        blockers,
    }
}

pub fn validate_promotion_artifact_identity_report(
    report: &PromotionArtifactIdentityReportV1,
) -> Result<(), PromotionArtifactIdentityReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            PromotionArtifactIdentityReportError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: report.schema_version,
            },
        );
    }
    ensure_identity_report_field("report_id", &report.report_id)?;
    ensure_identity_report_status_matches_blockers(report)?;
    ensure_unique_artifact_identity_roles(&report.roles)?;
    for role in &report.roles {
        validate_role_artifact_identity(role)?;
    }
    validate_artifact_identity_groups(&report.roles, &report.identity_groups)?;
    validate_artifact_identity_summary(report)?;
    validate_identity_report_blockers(&report.blockers)?;
    Ok(())
}

pub fn validate_promotion_wasm_store_identity_report(
    report: &PromotionWasmStoreIdentityReportV1,
) -> Result<(), PromotionWasmStoreIdentityReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            PromotionWasmStoreIdentityReportError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: report.schema_version,
            },
        );
    }
    ensure_wasm_store_identity_report_field("report_id", &report.report_id)?;
    ensure_wasm_store_identity_status_matches_blockers(report)?;
    ensure_unique_wasm_store_identity_roles(&report.roles)?;
    for role in &report.roles {
        validate_role_wasm_store_identity(role)?;
    }
    let expected_blockers = wasm_store_identity_blockers(&report.roles);
    if expected_blockers != report.blockers {
        return Err(PromotionWasmStoreIdentityReportError::BlockerMismatch);
    }
    validate_wasm_store_identity_blockers(&report.blockers)?;
    Ok(())
}

pub fn promotion_plan_transform_evidence(
    request: PromotionPlanTransformEvidenceRequest,
) -> Result<PromotionPlanTransformEvidenceV1, PromotionPlanTransformEvidenceError> {
    ensure_evidence_field("evidence_id", &request.evidence_id)?;
    ensure_evidence_field("generated_at", &request.generated_at)?;
    validate_promotion_plan_transform(&request.transform)?;
    let evidence = PromotionPlanTransformEvidenceV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        evidence_id: request.evidence_id,
        generated_at: request.generated_at,
        transform: request.transform,
    };
    validate_promotion_plan_transform_evidence(&evidence)?;
    Ok(evidence)
}

pub fn artifact_promotion_plan(
    request: ArtifactPromotionPlanRequest,
) -> Result<ArtifactPromotionPlanV1, ArtifactPromotionPlanError> {
    ensure_artifact_promotion_plan_field("plan_id", &request.plan_id)?;
    ensure_artifact_promotion_plan_field("generated_at", &request.generated_at)?;
    validate_promotion_readiness(&request.readiness)?;
    validate_promotion_artifact_identity_report(&request.artifact_identity_report)?;
    validate_promotion_plan_transform(&request.transform)?;
    if let Some(lineage) = &request.target_execution_lineage {
        validate_promotion_target_execution_lineage(lineage)?;
    }

    let blockers =
        artifact_promotion_plan_blockers(&request.readiness, &request.artifact_identity_report);
    let status = if blockers.is_empty() {
        PromotionReadinessStatusV1::Ready
    } else {
        PromotionReadinessStatusV1::Blocked
    };
    let plan = ArtifactPromotionPlanV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        plan_id: request.plan_id,
        generated_at: request.generated_at,
        status,
        target_plan_id: request.transform.target_plan_id.clone(),
        promoted_plan_id: request.transform.promoted_plan_id.clone(),
        promotion_plan_lineage_digest: request.transform.promotion_plan_lineage_digest.clone(),
        readiness: request.readiness,
        artifact_identity_report: request.artifact_identity_report,
        transform: request.transform,
        target_execution_lineage: request.target_execution_lineage,
        blockers,
    };
    validate_artifact_promotion_plan(&plan)?;
    Ok(plan)
}

pub fn promotion_target_execution_lineage(
    request: PromotionTargetExecutionLineageRequest,
) -> Result<PromotionTargetExecutionLineageV1, PromotionTargetExecutionLineageError> {
    ensure_target_execution_lineage_field("lineage_id", &request.lineage_id)?;
    ensure_target_execution_lineage_field("generated_at", &request.generated_at)?;
    validate_promotion_plan_transform(&request.transform)?;
    validate_deployment_execution_preflight(&request.execution_preflight)?;

    let target_execution_lineage_digest = promotion_target_execution_lineage_digest(
        &request.transform,
        &request.execution_preflight,
        false,
    );
    let lineage = PromotionTargetExecutionLineageV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        lineage_id: request.lineage_id,
        generated_at: request.generated_at,
        target_execution_lineage_digest,
        transform: request.transform,
        execution_preflight: request.execution_preflight,
        execution_attempted: false,
    };
    validate_promotion_target_execution_lineage(&lineage)?;
    Ok(lineage)
}

pub fn validate_artifact_promotion_plan(
    plan: &ArtifactPromotionPlanV1,
) -> Result<(), ArtifactPromotionPlanError> {
    if plan.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(ArtifactPromotionPlanError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            found: plan.schema_version,
        });
    }
    ensure_artifact_promotion_plan_field("plan_id", &plan.plan_id)?;
    ensure_artifact_promotion_plan_field("generated_at", &plan.generated_at)?;
    ensure_artifact_promotion_plan_field("target_plan_id", &plan.target_plan_id)?;
    ensure_artifact_promotion_plan_field("promoted_plan_id", &plan.promoted_plan_id)?;
    ensure_artifact_promotion_plan_field(
        "promotion_plan_lineage_digest",
        &plan.promotion_plan_lineage_digest,
    )?;
    ensure_artifact_promotion_status_matches_blockers(plan)?;
    validate_promotion_readiness(&plan.readiness)?;
    validate_promotion_artifact_identity_report(&plan.artifact_identity_report)?;
    validate_promotion_plan_transform(&plan.transform)?;
    ensure_artifact_promotion_plan_linkage(plan)?;
    if let Some(lineage) = &plan.target_execution_lineage {
        validate_promotion_target_execution_lineage(lineage)?;
        if lineage.transform != plan.transform {
            return Err(ArtifactPromotionPlanError::LinkageMismatch {
                field: "target_execution_lineage.transform",
            });
        }
    }
    Ok(())
}

pub fn validate_artifact_promotion_plan_for_check(
    plan: &ArtifactPromotionPlanV1,
    target_check: &DeploymentCheckV1,
) -> Result<(), ArtifactPromotionPlanError> {
    validate_artifact_promotion_plan(plan)?;
    if target_check.plan != plan.transform.promoted_plan {
        return Err(ArtifactPromotionPlanError::LinkageMismatch {
            field: "target_check.plan",
        });
    }
    let Some(lineage) = &plan.target_execution_lineage else {
        return Err(ArtifactPromotionPlanError::MissingTargetExecutionLineage);
    };
    validate_deployment_execution_preflight_for_check(target_check, &lineage.execution_preflight)
        .map_err(ArtifactPromotionPlanError::TargetCheck)?;
    Ok(())
}

pub fn artifact_promotion_provenance_report(
    request: ArtifactPromotionProvenanceReportRequest,
) -> Result<ArtifactPromotionProvenanceReportV1, ArtifactPromotionProvenanceReportError> {
    ensure_provenance_report_field("report_id", &request.report_id)?;
    validate_artifact_promotion_plan(&request.artifact_promotion_plan)?;
    if let Some(report) = &request.wasm_store_identity_report {
        validate_promotion_wasm_store_identity_report(report)?;
    }
    if let Some(report) = &request.materialization_identity_report {
        validate_promotion_materialization_identity_report(report)?;
    }
    let report = build_artifact_promotion_provenance_report(request);
    validate_artifact_promotion_provenance_report(&report)?;
    Ok(report)
}

pub fn validate_artifact_promotion_provenance_report(
    report: &ArtifactPromotionProvenanceReportV1,
) -> Result<(), ArtifactPromotionProvenanceReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            ArtifactPromotionProvenanceReportError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: report.schema_version,
            },
        );
    }
    ensure_provenance_report_field("report_id", &report.report_id)?;
    ensure_provenance_report_field(
        "artifact_promotion_plan_id",
        &report.artifact_promotion_plan_id,
    )?;
    ensure_provenance_report_field("target_plan_id", &report.target_plan_id)?;
    ensure_provenance_report_field("promoted_plan_id", &report.promoted_plan_id)?;
    ensure_provenance_report_field(
        "promotion_plan_lineage_digest",
        &report.promotion_plan_lineage_digest,
    )?;
    ensure_provenance_report_field("readiness_id", &report.readiness_id)?;
    ensure_provenance_report_field(
        "artifact_identity_report_id",
        &report.artifact_identity_report_id,
    )?;
    ensure_provenance_report_field("transform_id", &report.transform_id)?;
    if let Some(lineage_id) = &report.target_execution_lineage_id {
        ensure_provenance_report_field("target_execution_lineage_id", lineage_id)?;
    }
    if let Some(report_id) = &report.wasm_store_identity_report_id {
        ensure_provenance_report_field("wasm_store_identity_report_id", report_id)?;
    }
    if let Some(report_id) = &report.materialization_identity_report_id {
        ensure_provenance_report_field("materialization_identity_report_id", report_id)?;
    }
    ensure_provenance_report_status_matches_blockers(report)?;
    ensure_unique_provenance_roles(&report.roles)?;
    for role in &report.roles {
        validate_role_promotion_provenance(role)?;
    }
    validate_provenance_report_blockers(&report.blockers)?;
    Ok(())
}

pub fn artifact_promotion_execution_receipt(
    request: ArtifactPromotionExecutionReceiptRequest,
) -> Result<ArtifactPromotionExecutionReceiptV1, ArtifactPromotionExecutionReceiptError> {
    ensure_execution_receipt_field("receipt_id", &request.receipt_id)?;
    validate_artifact_promotion_provenance_report(&request.provenance_report)?;
    ensure_execution_receipt_provenance_ready(request.provenance_report.status)?;
    validate_deployment_receipt_for_promotion(
        &request.deployment_receipt,
        &request.provenance_report,
    )?;
    let receipt = build_artifact_promotion_execution_receipt(request);
    validate_artifact_promotion_execution_receipt(&receipt)?;
    Ok(receipt)
}

pub fn validate_artifact_promotion_execution_receipt(
    receipt: &ArtifactPromotionExecutionReceiptV1,
) -> Result<(), ArtifactPromotionExecutionReceiptError> {
    if receipt.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            ArtifactPromotionExecutionReceiptError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: receipt.schema_version,
            },
        );
    }
    ensure_execution_receipt_field("receipt_id", &receipt.receipt_id)?;
    ensure_execution_receipt_field(
        "artifact_promotion_plan_id",
        &receipt.artifact_promotion_plan_id,
    )?;
    ensure_execution_receipt_field("provenance_report_id", &receipt.provenance_report_id)?;
    ensure_execution_receipt_provenance_ready(receipt.provenance_status)?;
    ensure_execution_receipt_field("promoted_plan_id", &receipt.promoted_plan_id)?;
    ensure_execution_receipt_field(
        "promotion_plan_lineage_digest",
        &receipt.promotion_plan_lineage_digest,
    )?;
    ensure_execution_receipt_field("operation_id", &receipt.operation_id)?;
    ensure_execution_receipt_field("started_at", &receipt.started_at)?;
    if let Some(finished_at) = &receipt.finished_at {
        ensure_execution_receipt_field("finished_at", finished_at)?;
    }
    ensure_execution_receipt_linkage(receipt)
}

pub fn validate_promotion_plan_transform_evidence(
    evidence: &PromotionPlanTransformEvidenceV1,
) -> Result<(), PromotionPlanTransformEvidenceError> {
    if evidence.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(PromotionPlanTransformEvidenceError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            found: evidence.schema_version,
        });
    }
    ensure_evidence_field("evidence_id", &evidence.evidence_id)?;
    ensure_evidence_field("generated_at", &evidence.generated_at)?;
    validate_promotion_plan_transform(&evidence.transform)?;
    Ok(())
}

pub fn validate_promotion_target_execution_lineage(
    lineage: &PromotionTargetExecutionLineageV1,
) -> Result<(), PromotionTargetExecutionLineageError> {
    if lineage.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            PromotionTargetExecutionLineageError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: lineage.schema_version,
            },
        );
    }
    ensure_target_execution_lineage_field("lineage_id", &lineage.lineage_id)?;
    ensure_target_execution_lineage_field("generated_at", &lineage.generated_at)?;
    ensure_target_execution_lineage_sha256(
        "target_execution_lineage_digest",
        &lineage.target_execution_lineage_digest,
    )?;
    validate_promotion_plan_transform(&lineage.transform)?;
    validate_deployment_execution_preflight(&lineage.execution_preflight)?;
    if lineage.execution_attempted {
        return Err(PromotionTargetExecutionLineageError::ExecutionAttempted);
    }
    if lineage.execution_preflight.plan_id != lineage.transform.promoted_plan_id {
        return Err(PromotionTargetExecutionLineageError::LinkageMismatch {
            field: "execution_preflight.plan_id",
        });
    }
    let expected = promotion_target_execution_lineage_digest(
        &lineage.transform,
        &lineage.execution_preflight,
        lineage.execution_attempted,
    );
    if expected != lineage.target_execution_lineage_digest {
        return Err(PromotionTargetExecutionLineageError::LinkageMismatch {
            field: "target_execution_lineage_digest",
        });
    }
    Ok(())
}

pub fn validate_promotion_plan_transform(
    transform: &PromotionPlanTransformV1,
) -> Result<(), PromotionPlanTransformError> {
    if transform.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(PromotionPlanTransformError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            found: transform.schema_version,
        });
    }
    ensure_transform_field("transform_id", &transform.transform_id)?;
    ensure_transform_field("target_plan_id", &transform.target_plan_id)?;
    ensure_transform_field("promoted_plan_id", &transform.promoted_plan_id)?;
    ensure_transform_field(
        "promotion_plan_lineage_digest",
        &transform.promotion_plan_lineage_digest,
    )?;
    ensure_transform_field("promoted_plan.plan_id", &transform.promoted_plan.plan_id)?;
    if transform.promoted_plan.plan_id != transform.promoted_plan_id {
        return Err(PromotionPlanTransformError::PromotedPlanIdMismatch {
            expected: transform.promoted_plan_id.clone(),
            found: transform.promoted_plan.plan_id.clone(),
        });
    }
    ensure_unique_transform_roles(&transform.roles)?;
    for role in &transform.roles {
        validate_role_plan_transform(role, &transform.promoted_plan)?;
    }
    let expected = promotion_plan_lineage_digest(
        &transform.target_plan_id,
        &transform.promoted_plan_id,
        &transform.promoted_plan,
        &transform.roles,
    );
    if expected != transform.promotion_plan_lineage_digest {
        return Err(PromotionPlanTransformError::RoleStateMismatch {
            role: "promotion_plan_lineage".to_string(),
            field: "promotion_plan_lineage_digest",
        });
    }
    Ok(())
}

#[must_use]
pub fn promotion_readiness_from_inputs(
    readiness_id: impl Into<String>,
    target_plan: &DeploymentPlanV1,
    inputs: &[RolePromotionInputV1],
) -> PromotionReadinessV1 {
    let mut roles = Vec::with_capacity(inputs.len());
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    for input in inputs {
        let target_artifact = target_plan
            .role_artifacts
            .iter()
            .find(|artifact| artifact.role == input.role);
        let Some(target_artifact) = target_artifact else {
            blockers.push(promotion_finding(
                "promotion_target_role_missing",
                format!("target plan does not contain role {}", input.role),
                SafetySeverityV1::HardFailure,
                &input.role,
            ));
            continue;
        };

        let role_readiness = role_promotion_readiness(input, target_artifact);
        collect_role_findings(input, &role_readiness, &mut blockers, &mut warnings);
        roles.push(role_readiness);
    }

    let status = if blockers.is_empty() {
        PromotionReadinessStatusV1::Ready
    } else {
        PromotionReadinessStatusV1::Blocked
    };

    PromotionReadinessV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        readiness_id: readiness_id.into(),
        target_plan_id: target_plan.plan_id.clone(),
        status,
        roles,
        blockers,
        warnings,
    }
}

#[must_use]
pub fn promotion_readiness_from_inputs_with_policy(
    readiness_id: impl Into<String>,
    target_plan: &DeploymentPlanV1,
    inputs: &[RolePromotionInputV1],
    policies: &[RolePromotionPolicyV1],
) -> PromotionReadinessV1 {
    let readiness_id = readiness_id.into();
    let policy_check =
        promotion_policy_check_from_inputs(format!("{readiness_id}:policy"), inputs, policies);
    let mut readiness = promotion_readiness_from_inputs(readiness_id, target_plan, inputs);
    readiness.blockers.extend(policy_check.blockers);
    readiness.status = if readiness.blockers.is_empty() {
        PromotionReadinessStatusV1::Ready
    } else {
        PromotionReadinessStatusV1::Blocked
    };
    readiness
}

pub fn validate_promotion_readiness(
    readiness: &PromotionReadinessV1,
) -> Result<(), PromotionReadinessError> {
    if readiness.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(PromotionReadinessError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            found: readiness.schema_version,
        });
    }
    ensure_readiness_field("readiness_id", &readiness.readiness_id)?;
    ensure_readiness_field("target_plan_id", &readiness.target_plan_id)?;
    ensure_readiness_status_matches_blockers(readiness)?;
    ensure_unique_readiness_roles(&readiness.roles)?;
    for role in &readiness.roles {
        validate_role_readiness(role)?;
    }
    validate_readiness_findings(
        "blockers",
        &readiness.blockers,
        SafetySeverityV1::HardFailure,
    )?;
    validate_readiness_findings("warnings", &readiness.warnings, SafetySeverityV1::Warning)?;
    Ok(())
}

pub fn validate_role_artifact_source(
    source: &RoleArtifactSourceV1,
) -> Result<(), PromotionArtifactSourceError> {
    ensure_field("role", &source.role)?;
    ensure_locator_requirement(source)?;
    ensure_previous_receipt_requirement(source)?;
    ensure_digest_requirement(source)?;
    ensure_previous_receipt_lineage_digest_requirement(source)?;
    ensure_optional_sha256(
        "expected_wasm_sha256",
        source.expected_wasm_sha256.as_deref(),
    )?;
    ensure_optional_sha256(
        "expected_wasm_gz_sha256",
        source.expected_wasm_gz_sha256.as_deref(),
    )?;
    ensure_optional_sha256(
        "expected_candid_sha256",
        source.expected_candid_sha256.as_deref(),
    )?;
    ensure_optional_sha256(
        "expected_canonical_embedded_config_sha256",
        source.expected_canonical_embedded_config_sha256.as_deref(),
    )?;
    ensure_optional_sha256(
        "previous_receipt_lineage_digest",
        source.previous_receipt_lineage_digest.as_deref(),
    )?;
    Ok(())
}

pub fn validate_role_promotion_policy(
    policy: &RolePromotionPolicyV1,
) -> Result<(), PromotionPolicyCheckError> {
    ensure_policy_field("role", &policy.role)?;
    if policy.allowed_promotion_levels.is_empty() {
        return Err(PromotionPolicyCheckError::EmptyAllowedLevels {
            role: policy.role.clone(),
        });
    }
    let mut seen = BTreeSet::new();
    for level in &policy.allowed_promotion_levels {
        if !seen.insert(*level) {
            return Err(PromotionPolicyCheckError::DuplicateAllowedLevel {
                role: policy.role.clone(),
                level: *level,
            });
        }
    }
    let mut seen_requirements = BTreeSet::new();
    for requirement in &policy.requirements {
        if !seen_requirements.insert(*requirement) {
            return Err(PromotionPolicyCheckError::DecisionMismatch {
                role: policy.role.clone(),
                field: "requirements",
            });
        }
    }
    if policy
        .requirements
        .contains(&PromotionPolicyRequirementV1::SealedBytes)
        && policy
            .allowed_promotion_levels
            .iter()
            .any(|level| *level != PromotionArtifactLevelV1::SealedWasm)
    {
        return Err(PromotionPolicyCheckError::DecisionMismatch {
            role: policy.role.clone(),
            field: "sealed_bytes",
        });
    }
    Ok(())
}

pub fn build_materialization_evidence(
    request: BuildMaterializationEvidenceRequest,
) -> Result<BuildMaterializationEvidenceV1, PromotionMaterializationIdentityError> {
    ensure_materialization_field("evidence_id", &request.evidence_id)?;
    validate_build_recipe_identity(&request.recipe)?;
    validate_build_materialization_input(&request.materialization_input)?;
    validate_build_materialization_result(&request.materialization_result)?;
    let computed_materialization_input_digest =
        build_materialization_input_digest(&request.materialization_input);
    let evidence = BuildMaterializationEvidenceV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        evidence_id: request.evidence_id,
        recipe_id_matches_input: request.recipe.recipe_id
            == request.materialization_input.build_recipe_id,
        recipe_id_matches_result: request.recipe.recipe_id
            == request.materialization_result.build_recipe_id,
        materialization_input_digest_matches_result: computed_materialization_input_digest
            == request.materialization_result.materialization_input_digest,
        computed_materialization_input_digest,
        recipe: request.recipe,
        materialization_input: request.materialization_input,
        materialization_result: request.materialization_result,
    };
    validate_build_materialization_evidence(&evidence)?;
    Ok(evidence)
}

pub fn validate_build_materialization_evidence(
    evidence: &BuildMaterializationEvidenceV1,
) -> Result<(), PromotionMaterializationIdentityError> {
    if evidence.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            PromotionMaterializationIdentityError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: evidence.schema_version,
            },
        );
    }
    ensure_materialization_field("evidence_id", &evidence.evidence_id)?;
    validate_build_recipe_identity(&evidence.recipe)?;
    validate_build_materialization_input(&evidence.materialization_input)?;
    validate_build_materialization_result(&evidence.materialization_result)?;
    ensure_materialization_sha256(
        "computed_materialization_input_digest",
        &evidence.computed_materialization_input_digest,
    )?;
    ensure_materialization_link(
        "recipe_id_matches_input",
        evidence.recipe_id_matches_input
            == (evidence.recipe.recipe_id == evidence.materialization_input.build_recipe_id),
    )?;
    ensure_materialization_link("recipe_id_matches_input", evidence.recipe_id_matches_input)?;
    ensure_materialization_link(
        "recipe_id_matches_result",
        evidence.recipe_id_matches_result
            == (evidence.recipe.recipe_id == evidence.materialization_result.build_recipe_id),
    )?;
    ensure_materialization_link(
        "recipe_id_matches_result",
        evidence.recipe_id_matches_result,
    )?;
    let computed = build_materialization_input_digest(&evidence.materialization_input);
    if computed != evidence.computed_materialization_input_digest {
        return Err(PromotionMaterializationIdentityError::DigestMismatch {
            field: "computed_materialization_input_digest",
            expected: computed,
            found: evidence.computed_materialization_input_digest.clone(),
        });
    }
    ensure_materialization_link(
        "materialization_input_digest_matches_result",
        evidence.materialization_input_digest_matches_result
            == (evidence.computed_materialization_input_digest
                == evidence.materialization_result.materialization_input_digest),
    )?;
    ensure_materialization_link(
        "materialization_input_digest_matches_result",
        evidence.materialization_input_digest_matches_result,
    )?;
    Ok(())
}

pub fn promotion_materialization_identity_report_from_evidence(
    request: PromotionMaterializationIdentityReportRequest,
) -> Result<PromotionMaterializationIdentityReportV1, PromotionMaterializationIdentityReportError> {
    ensure_materialization_report_field("report_id", &request.report_id)?;
    for evidence in &request.evidence {
        validate_build_materialization_evidence(evidence)?;
    }
    let report = promotion_materialization_identity_report(&request.report_id, &request.evidence);
    validate_promotion_materialization_identity_report(&report)?;
    Ok(report)
}

#[must_use]
pub fn promotion_materialization_identity_report(
    report_id: impl Into<String>,
    evidence: &[BuildMaterializationEvidenceV1],
) -> PromotionMaterializationIdentityReportV1 {
    let roles = evidence
        .iter()
        .map(role_materialization_identity_from_evidence)
        .collect::<Vec<_>>();
    let output_groups = promotion_materialization_output_groups(&roles);
    let blockers = Vec::new();
    PromotionMaterializationIdentityReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: report_id.into(),
        status: PromotionReadinessStatusV1::Ready,
        roles,
        output_groups,
        blockers,
    }
}

pub fn validate_promotion_materialization_identity_report(
    report: &PromotionMaterializationIdentityReportV1,
) -> Result<(), PromotionMaterializationIdentityReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            PromotionMaterializationIdentityReportError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: report.schema_version,
            },
        );
    }
    ensure_materialization_report_field("report_id", &report.report_id)?;
    ensure_materialization_report_status_matches_blockers(report)?;
    ensure_unique_materialization_report_roles(&report.roles)?;
    for role in &report.roles {
        validate_role_materialization_identity(role)?;
    }
    validate_materialization_output_groups(&report.roles, &report.output_groups)?;
    let expected_blockers = Vec::<SafetyFindingV1>::new();
    if report.blockers != expected_blockers {
        return Err(PromotionMaterializationIdentityReportError::BlockerMismatch);
    }
    validate_materialization_report_blockers(&report.blockers)?;
    Ok(())
}

#[must_use]
pub fn build_materialization_input_digest(input: &BuildMaterializationInputV1) -> String {
    stable_json_sha256_hex(input)
}

pub fn validate_build_recipe_identity(
    recipe: &BuildRecipeIdentityV1,
) -> Result<(), PromotionMaterializationIdentityError> {
    ensure_materialization_field("recipe_id", &recipe.recipe_id)?;
    ensure_materialization_field("source_revision", &recipe.source_revision)?;
    ensure_materialization_field("package_or_role_selector", &recipe.package_or_role_selector)?;
    ensure_materialization_field("cargo_profile", &recipe.cargo_profile)?;
    ensure_materialization_sha256("cargo_features_digest", &recipe.cargo_features_digest)?;
    ensure_materialization_sha256("cargo_lock_digest", &recipe.cargo_lock_digest)?;
    ensure_materialization_field("rust_toolchain", &recipe.rust_toolchain)?;
    ensure_materialization_field("builder_version", &recipe.builder_version)?;
    ensure_materialization_field("target_triple", &recipe.target_triple)?;
    ensure_materialization_field("linker_identity", &recipe.linker_identity)?;
    ensure_materialization_field("deterministic_build_mode", &recipe.deterministic_build_mode)?;
    ensure_materialization_field("wasm_opt_version", &recipe.wasm_opt_version)?;
    ensure_materialization_field("compression_identity", &recipe.compression_identity)?;
    Ok(())
}

pub fn validate_build_materialization_input(
    input: &BuildMaterializationInputV1,
) -> Result<(), PromotionMaterializationIdentityError> {
    ensure_materialization_field("materialization_input_id", &input.materialization_input_id)?;
    ensure_materialization_field("build_recipe_id", &input.build_recipe_id)?;
    ensure_materialization_sha256(
        "canonical_embedded_config_sha256",
        &input.canonical_embedded_config_sha256,
    )?;
    ensure_materialization_field("network", &input.network)?;
    ensure_materialization_field("root_trust_anchor", &input.root_trust_anchor)?;
    ensure_materialization_field("runtime_variant", &input.runtime_variant)?;
    Ok(())
}

pub fn validate_build_materialization_result(
    result: &BuildMaterializationResultV1,
) -> Result<(), PromotionMaterializationIdentityError> {
    ensure_materialization_field(
        "materialization_result_id",
        &result.materialization_result_id,
    )?;
    ensure_materialization_field("build_recipe_id", &result.build_recipe_id)?;
    ensure_materialization_sha256(
        "materialization_input_digest",
        &result.materialization_input_digest,
    )?;
    ensure_materialization_sha256("wasm_sha256", &result.wasm_sha256)?;
    ensure_materialization_sha256("wasm_gz_sha256", &result.wasm_gz_sha256)?;
    ensure_materialization_sha256("installed_module_hash", &result.installed_module_hash)?;
    ensure_materialization_sha256("candid_sha256", &result.candid_sha256)?;
    Ok(())
}

fn apply_promotion_input_to_role_artifact(
    role_artifact: &mut RoleArtifactV1,
    input: &RolePromotionInputV1,
) {
    match input.promotion_level {
        PromotionArtifactLevelV1::SealedWasm => {
            role_artifact.source = artifact_source_for_promotion_source(input.source.kind);
            apply_promotion_source_locator(role_artifact, &input.source);
            role_artifact
                .wasm_sha256
                .clone_from(&input.source.expected_wasm_sha256);
            role_artifact
                .wasm_gz_sha256
                .clone_from(&input.source.expected_wasm_gz_sha256);
            role_artifact
                .candid_sha256
                .clone_from(&input.source.expected_candid_sha256);
            role_artifact
                .canonical_embedded_config_sha256
                .clone_from(&input.source.expected_canonical_embedded_config_sha256);
        }
        PromotionArtifactLevelV1::SourceBuild => {}
    }
}

const fn artifact_source_for_promotion_source(kind: RoleArtifactSourceKindV1) -> ArtifactSourceV1 {
    match kind {
        RoleArtifactSourceKindV1::WorkspacePackage => ArtifactSourceV1::LocalBuild,
        RoleArtifactSourceKindV1::CanonicalWasmStoreDefault => ArtifactSourceV1::WasmStore,
        RoleArtifactSourceKindV1::PublishedPackage
        | RoleArtifactSourceKindV1::LocalWasm
        | RoleArtifactSourceKindV1::LocalWasmGz
        | RoleArtifactSourceKindV1::PreviousReceiptArtifact => ArtifactSourceV1::External,
    }
}

fn apply_promotion_source_locator(
    role_artifact: &mut RoleArtifactV1,
    source: &RoleArtifactSourceV1,
) {
    match source.kind {
        RoleArtifactSourceKindV1::LocalWasm => {
            role_artifact.wasm_path.clone_from(&source.locator);
        }
        RoleArtifactSourceKindV1::LocalWasmGz => {
            role_artifact.wasm_gz_path.clone_from(&source.locator);
        }
        _ => {}
    }
}

fn promotion_plan_transform_from_parts(
    target_plan: &DeploymentPlanV1,
    promoted_plan: DeploymentPlanV1,
    inputs: &[RolePromotionInputV1],
) -> PromotionPlanTransformV1 {
    let roles = inputs
        .iter()
        .filter_map(|input| {
            let before = target_plan
                .role_artifacts
                .iter()
                .find(|artifact| artifact.role == input.role)?;
            let after = promoted_plan
                .role_artifacts
                .iter()
                .find(|artifact| artifact.role == input.role)?;
            Some(role_plan_transform(input, before, after))
        })
        .collect::<Vec<_>>();
    let promotion_plan_lineage_digest = promotion_plan_lineage_digest(
        &target_plan.plan_id,
        &promoted_plan.plan_id,
        &promoted_plan,
        &roles,
    );

    PromotionPlanTransformV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        transform_id: format!("promotion-transform:{}", promoted_plan.plan_id),
        target_plan_id: target_plan.plan_id.clone(),
        promoted_plan_id: promoted_plan.plan_id.clone(),
        promotion_plan_lineage_digest,
        promoted_plan,
        roles,
    }
}

#[must_use]
pub fn promotion_plan_lineage_digest(
    target_plan_id: &str,
    promoted_plan_id: &str,
    promoted_plan: &DeploymentPlanV1,
    roles: &[RolePromotionPlanTransformV1],
) -> String {
    stable_json_sha256_hex(&PromotionPlanLineageInput {
        target_plan_id,
        promoted_plan_id,
        promoted_plan,
        roles,
    })
}

#[must_use]
pub fn promotion_target_execution_lineage_digest(
    transform: &PromotionPlanTransformV1,
    preflight: &DeploymentExecutionPreflightV1,
    execution_attempted: bool,
) -> String {
    stable_json_sha256_hex(&PromotionTargetExecutionLineageInput {
        promotion_plan_lineage_digest: &transform.promotion_plan_lineage_digest,
        promoted_plan_id: &transform.promoted_plan_id,
        preflight_plan_id: &preflight.plan_id,
        preflight_safety_report_id: &preflight.safety_report_id,
        preflight_authority_plan_id: &preflight.authority_plan_id,
        preflight_backend: &preflight.backend,
        preflight_status: preflight.status,
        planned_phases: &preflight.planned_phases,
        required_capabilities: &preflight.required_capabilities,
        missing_capabilities: &preflight.missing_capabilities,
        execution_attempted,
    })
}

fn role_plan_transform(
    input: &RolePromotionInputV1,
    before: &RoleArtifactV1,
    after: &RoleArtifactV1,
) -> RolePromotionPlanTransformV1 {
    RolePromotionPlanTransformV1 {
        role: input.role.clone(),
        promotion_level: input.promotion_level,
        source_kind: input.source.kind,
        source_locator: input.source.locator.clone(),
        artifact_source_before: before.source,
        artifact_source_after: after.source,
        wasm_sha256_before: before.wasm_sha256.clone(),
        wasm_sha256_after: after.wasm_sha256.clone(),
        wasm_gz_sha256_before: before.wasm_gz_sha256.clone(),
        wasm_gz_sha256_after: after.wasm_gz_sha256.clone(),
        candid_sha256_before: before.candid_sha256.clone(),
        candid_sha256_after: after.candid_sha256.clone(),
        canonical_embedded_config_sha256_before: before.canonical_embedded_config_sha256.clone(),
        canonical_embedded_config_sha256_after: after.canonical_embedded_config_sha256.clone(),
        artifact_identity_changed: artifact_identity_changed(before, after),
        embedded_config_changed: before.canonical_embedded_config_sha256
            != after.canonical_embedded_config_sha256,
        target_materialization_preserved: input.promotion_level
            == PromotionArtifactLevelV1::SourceBuild
            && role_materialization_identity_matches(before, after),
        source_build_materialization: None,
    }
}

fn attach_source_build_materialization(
    transform: &mut PromotionPlanTransformV1,
    inputs: &[RolePromotionInputV1],
    evidence: &[BuildMaterializationEvidenceV1],
) -> Result<(), PromotionPlanTransformError> {
    let input_roles = inputs
        .iter()
        .map(|input| input.role.as_str())
        .collect::<BTreeSet<_>>();
    let mut links = BTreeMap::new();
    for item in evidence {
        validate_build_materialization_evidence(item)?;
        let role = item.recipe.package_or_role_selector.as_str();
        if !input_roles.contains(role) {
            return Err(PromotionPlanTransformError::UnexpectedMaterializationRole {
                role: role.to_string(),
            });
        }
        if links
            .insert(role.to_string(), materialization_link_from_evidence(item))
            .is_some()
        {
            return Err(PromotionPlanTransformError::DuplicateMaterializationRole {
                role: role.to_string(),
            });
        }
    }

    for role in &mut transform.roles {
        match role.promotion_level {
            PromotionArtifactLevelV1::SourceBuild => {
                let Some(link) = links.remove(&role.role) else {
                    return Err(PromotionPlanTransformError::MaterializationRoleMissing {
                        role: role.role.clone(),
                    });
                };
                role.source_build_materialization = Some(link);
            }
            PromotionArtifactLevelV1::SealedWasm => {
                if links.remove(&role.role).is_some() {
                    return Err(PromotionPlanTransformError::UnexpectedMaterializationRole {
                        role: role.role.clone(),
                    });
                }
            }
        }
    }

    if let Some(role) = links.keys().next() {
        return Err(PromotionPlanTransformError::UnexpectedMaterializationRole {
            role: role.clone(),
        });
    }
    Ok(())
}

fn materialization_link_from_evidence(
    evidence: &BuildMaterializationEvidenceV1,
) -> RolePromotionMaterializationLinkV1 {
    RolePromotionMaterializationLinkV1 {
        role: evidence.recipe.package_or_role_selector.clone(),
        evidence_id: evidence.evidence_id.clone(),
        recipe_id: evidence.recipe.recipe_id.clone(),
        materialization_input_id: evidence
            .materialization_input
            .materialization_input_id
            .clone(),
        materialization_result_id: evidence
            .materialization_result
            .materialization_result_id
            .clone(),
        materialization_input_digest: evidence.computed_materialization_input_digest.clone(),
        wasm_sha256: evidence.materialization_result.wasm_sha256.clone(),
        wasm_gz_sha256: evidence.materialization_result.wasm_gz_sha256.clone(),
        installed_module_hash: evidence
            .materialization_result
            .installed_module_hash
            .clone(),
        candid_sha256: evidence.materialization_result.candid_sha256.clone(),
    }
}

fn artifact_promotion_plan_blockers(
    readiness: &PromotionReadinessV1,
    artifact_identity_report: &PromotionArtifactIdentityReportV1,
) -> Vec<SafetyFindingV1> {
    let mut blockers =
        Vec::with_capacity(readiness.blockers.len() + artifact_identity_report.blockers.len());
    blockers.extend(readiness.blockers.clone());
    blockers.extend(artifact_identity_report.blockers.clone());
    blockers
}

fn build_artifact_promotion_provenance_report(
    request: ArtifactPromotionProvenanceReportRequest,
) -> ArtifactPromotionProvenanceReportV1 {
    let plan = request.artifact_promotion_plan;
    let mut roles = plan
        .transform
        .roles
        .iter()
        .map(role_promotion_provenance_from_transform)
        .collect::<Vec<_>>();
    attach_wasm_store_provenance(&mut roles, request.wasm_store_identity_report.as_ref());
    attach_materialization_provenance(&mut roles, request.materialization_identity_report.as_ref());
    let blockers = artifact_promotion_provenance_blockers(
        &plan,
        request.wasm_store_identity_report.as_ref(),
        request.materialization_identity_report.as_ref(),
        &roles,
    );
    let status = if blockers.is_empty() {
        PromotionReadinessStatusV1::Ready
    } else {
        PromotionReadinessStatusV1::Blocked
    };
    ArtifactPromotionProvenanceReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: request.report_id,
        status,
        artifact_promotion_plan_id: plan.plan_id,
        target_plan_id: plan.target_plan_id,
        promoted_plan_id: plan.promoted_plan_id,
        promotion_plan_lineage_digest: plan.promotion_plan_lineage_digest,
        readiness_id: plan.readiness.readiness_id,
        artifact_identity_report_id: plan.artifact_identity_report.report_id,
        transform_id: plan.transform.transform_id,
        target_execution_lineage_id: plan
            .target_execution_lineage
            .map(|lineage| lineage.lineage_id),
        wasm_store_identity_report_id: request
            .wasm_store_identity_report
            .map(|report| report.report_id),
        materialization_identity_report_id: request
            .materialization_identity_report
            .map(|report| report.report_id),
        execution_attempted: false,
        roles,
        blockers,
    }
}

fn artifact_promotion_provenance_blockers(
    plan: &ArtifactPromotionPlanV1,
    wasm_store_report: Option<&PromotionWasmStoreIdentityReportV1>,
    materialization_report: Option<&PromotionMaterializationIdentityReportV1>,
    roles: &[RolePromotionProvenanceV1],
) -> Vec<SafetyFindingV1> {
    let mut blockers = plan.blockers.clone();
    if let Some(report) = wasm_store_report {
        blockers.extend(report.blockers.iter().cloned());
    }
    if let Some(report) = materialization_report {
        blockers.extend(report.blockers.iter().cloned());
    }
    let role_names = roles
        .iter()
        .map(|role| role.role.as_str())
        .collect::<BTreeSet<_>>();
    if let Some(report) = wasm_store_report {
        for role in &report.roles {
            if !role_names.contains(role.role.as_str()) {
                blockers.push(promotion_finding(
                    "promotion_provenance_unknown_wasm_store_role",
                    format!(
                        "wasm-store identity report contains unknown role {}",
                        role.role
                    ),
                    SafetySeverityV1::HardFailure,
                    &role.role,
                ));
            }
        }
    }
    if let Some(report) = materialization_report {
        for role in &report.roles {
            if !role_names.contains(role.role.as_str()) {
                blockers.push(promotion_finding(
                    "promotion_provenance_unknown_materialization_role",
                    format!(
                        "materialization identity report contains unknown role {}",
                        role.role
                    ),
                    SafetySeverityV1::HardFailure,
                    &role.role,
                ));
            }
        }
    }
    blockers
}

fn role_promotion_provenance_from_transform(
    role: &RolePromotionPlanTransformV1,
) -> RolePromotionProvenanceV1 {
    RolePromotionProvenanceV1 {
        role: role.role.clone(),
        promotion_level: role.promotion_level,
        source_kind: role.source_kind,
        artifact_identity_changed: role.artifact_identity_changed,
        embedded_config_changed: role.embedded_config_changed,
        target_materialization_preserved: role.target_materialization_preserved,
        materialization_evidence_id: role
            .source_build_materialization
            .as_ref()
            .map(|materialization| materialization.evidence_id.clone()),
        wasm_store_locator: None,
    }
}

fn attach_wasm_store_provenance(
    roles: &mut [RolePromotionProvenanceV1],
    report: Option<&PromotionWasmStoreIdentityReportV1>,
) {
    let Some(report) = report else {
        return;
    };
    for role in roles {
        if let Some(wasm_store_role) = report.roles.iter().find(|item| item.role == role.role) {
            role.wasm_store_locator = wasm_store_role.wasm_store_locator.clone();
        }
    }
}

fn attach_materialization_provenance(
    roles: &mut [RolePromotionProvenanceV1],
    report: Option<&PromotionMaterializationIdentityReportV1>,
) {
    let Some(report) = report else {
        return;
    };
    for role in roles {
        if let Some(materialization_role) = report.roles.iter().find(|item| item.role == role.role)
        {
            role.materialization_evidence_id = Some(materialization_role.evidence_id.clone());
        }
    }
}

fn build_artifact_promotion_execution_receipt(
    request: ArtifactPromotionExecutionReceiptRequest,
) -> ArtifactPromotionExecutionReceiptV1 {
    let roles = request
        .provenance_report
        .roles
        .iter()
        .map(|role| role_promotion_execution_receipt(role, &request.deployment_receipt))
        .collect::<Vec<_>>();
    ArtifactPromotionExecutionReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        receipt_id: request.receipt_id,
        artifact_promotion_plan_id: request.provenance_report.artifact_promotion_plan_id.clone(),
        provenance_report_id: request.provenance_report.report_id,
        provenance_status: request.provenance_report.status,
        promoted_plan_id: request.provenance_report.promoted_plan_id.clone(),
        promotion_plan_lineage_digest: request.provenance_report.promotion_plan_lineage_digest,
        operation_id: request.deployment_receipt.operation_id.clone(),
        operation_status: request.deployment_receipt.operation_status,
        command_result: request.deployment_receipt.command_result.clone(),
        started_at: request.deployment_receipt.started_at.clone(),
        finished_at: request.deployment_receipt.finished_at.clone(),
        deployment_receipt: request.deployment_receipt,
        roles,
    }
}

fn role_promotion_execution_receipt(
    role: &RolePromotionProvenanceV1,
    deployment_receipt: &DeploymentReceiptV1,
) -> RolePromotionExecutionReceiptV1 {
    let role_receipt = deployment_receipt
        .role_phase_receipts
        .iter()
        .rev()
        .find(|receipt| receipt.role == role.role);
    RolePromotionExecutionReceiptV1 {
        role: role.role.clone(),
        promotion_level: role.promotion_level,
        materialization_evidence_id: role.materialization_evidence_id.clone(),
        wasm_store_locator: role.wasm_store_locator.clone(),
        role_phase_result: role_receipt.map(|receipt| receipt.result),
        artifact_digest: role_receipt.and_then(|receipt| receipt.artifact_digest.clone()),
        observed_module_hash_after: role_receipt
            .and_then(|receipt| receipt.observed_module_hash_after.clone()),
        canonical_embedded_config_sha256: role_receipt
            .and_then(|receipt| receipt.canonical_embedded_config_sha256.clone()),
    }
}

fn validate_deployment_receipt_for_promotion(
    receipt: &DeploymentReceiptV1,
    provenance: &ArtifactPromotionProvenanceReportV1,
) -> Result<(), ArtifactPromotionExecutionReceiptError> {
    if receipt.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            ArtifactPromotionExecutionReceiptError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: receipt.schema_version,
            },
        );
    }
    ensure_execution_receipt_field("deployment_receipt.operation_id", &receipt.operation_id)?;
    ensure_execution_receipt_field("deployment_receipt.started_at", &receipt.started_at)?;
    if receipt.plan_id != provenance.promoted_plan_id {
        return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "deployment_receipt.plan_id",
        });
    }
    if let Some(finished_at) = &receipt.finished_at {
        ensure_execution_receipt_field("deployment_receipt.finished_at", finished_at)?;
    }
    Ok(())
}

fn wasm_store_identity_blockers(
    roles: &[RolePromotionWasmStoreIdentityV1],
) -> Vec<SafetyFindingV1> {
    let mut blockers = Vec::new();
    for role in roles {
        if role.transport != ArtifactTransportV1::WasmStore {
            blockers.push(promotion_finding(
                "promotion_wasm_store_transport_mismatch",
                format!("role {} was not staged through wasm_store", role.role),
                SafetySeverityV1::HardFailure,
                &role.role,
            ));
        }
        if role.wasm_store_locator.as_deref().is_none_or(str::is_empty) {
            blockers.push(promotion_finding(
                "promotion_wasm_store_locator_missing",
                format!("role {} does not record a wasm_store locator", role.role),
                SafetySeverityV1::HardFailure,
                &role.role,
            ));
        }
        if role.verified_postcondition.status != ObservationStatusV1::Observed {
            blockers.push(promotion_finding(
                "promotion_wasm_store_postcondition_not_observed",
                format!(
                    "role {} wasm_store postcondition is {:?}",
                    role.role, role.verified_postcondition.status
                ),
                SafetySeverityV1::HardFailure,
                &role.role,
            ));
        }
        if role.published_chunk_count != role.prepared_chunk_hashes.len() {
            blockers.push(promotion_finding(
                "promotion_wasm_store_chunk_count_mismatch",
                format!(
                    "role {} published {} chunk(s) for {} prepared chunk hash(es)",
                    role.role,
                    role.published_chunk_count,
                    role.prepared_chunk_hashes.len()
                ),
                SafetySeverityV1::HardFailure,
                &role.role,
            ));
        }
    }
    blockers
}

fn refresh_promotion_plan_lineage_digest(transform: &mut PromotionPlanTransformV1) {
    transform.promotion_plan_lineage_digest = promotion_plan_lineage_digest(
        &transform.target_plan_id,
        &transform.promoted_plan_id,
        &transform.promoted_plan,
        &transform.roles,
    );
}

fn artifact_identity_changed(before: &RoleArtifactV1, after: &RoleArtifactV1) -> bool {
    before.source != after.source
        || before.wasm_path != after.wasm_path
        || before.wasm_gz_path != after.wasm_gz_path
        || before.wasm_sha256 != after.wasm_sha256
        || before.wasm_gz_sha256 != after.wasm_gz_sha256
        || before.candid_path != after.candid_path
        || before.candid_sha256 != after.candid_sha256
}

fn role_materialization_identity_matches(before: &RoleArtifactV1, after: &RoleArtifactV1) -> bool {
    before.source == after.source
        && before.wasm_path == after.wasm_path
        && before.wasm_gz_path == after.wasm_gz_path
        && before.wasm_sha256 == after.wasm_sha256
        && before.wasm_gz_sha256 == after.wasm_gz_sha256
        && before.candid_path == after.candid_path
        && before.candid_sha256 == after.candid_sha256
        && before.canonical_embedded_config_sha256 == after.canonical_embedded_config_sha256
}

fn role_promotion_artifact_identity(
    input: &RolePromotionInputV1,
) -> RolePromotionArtifactIdentityV1 {
    let wasm_sha256 = input.source.expected_wasm_sha256.clone();
    let wasm_gz_sha256 = input.source.expected_wasm_gz_sha256.clone();
    RolePromotionArtifactIdentityV1 {
        role: input.role.clone(),
        promotion_level: input.promotion_level,
        source_kind: input.source.kind,
        source_locator: input.source.locator.clone(),
        identity_kind: promotion_artifact_identity_kind(input.promotion_level, &input.source),
        digest_pinned: wasm_sha256.is_some() || wasm_gz_sha256.is_some(),
        wasm_sha256,
        wasm_gz_sha256,
        candid_sha256: input.source.expected_candid_sha256.clone(),
        canonical_embedded_config_sha256: input
            .source
            .expected_canonical_embedded_config_sha256
            .clone(),
    }
}

fn role_wasm_store_identity_from_staging(
    receipt: &StagingReceiptV1,
) -> RolePromotionWasmStoreIdentityV1 {
    RolePromotionWasmStoreIdentityV1 {
        role: receipt.role.clone(),
        artifact_identity: receipt.artifact_identity.clone(),
        transport: receipt.transport,
        wasm_store_locator: receipt.wasm_store_locator.clone(),
        prepared_chunk_hashes: receipt.prepared_chunk_hashes.clone(),
        published_chunk_count: receipt.published_chunk_count,
        verified_postcondition: receipt.verified_postcondition.clone(),
    }
}

fn role_materialization_identity_from_evidence(
    evidence: &BuildMaterializationEvidenceV1,
) -> RolePromotionMaterializationIdentityV1 {
    RolePromotionMaterializationIdentityV1 {
        role: evidence.recipe.package_or_role_selector.clone(),
        evidence_id: evidence.evidence_id.clone(),
        recipe_id: evidence.recipe.recipe_id.clone(),
        materialization_input_id: evidence
            .materialization_input
            .materialization_input_id
            .clone(),
        materialization_result_id: evidence
            .materialization_result
            .materialization_result_id
            .clone(),
        materialization_input_digest: evidence.computed_materialization_input_digest.clone(),
        canonical_embedded_config_sha256: evidence
            .materialization_input
            .canonical_embedded_config_sha256
            .clone(),
        network: evidence.materialization_input.network.clone(),
        root_trust_anchor: evidence.materialization_input.root_trust_anchor.clone(),
        runtime_variant: evidence.materialization_input.runtime_variant.clone(),
        wasm_sha256: evidence.materialization_result.wasm_sha256.clone(),
        wasm_gz_sha256: evidence.materialization_result.wasm_gz_sha256.clone(),
        installed_module_hash: evidence
            .materialization_result
            .installed_module_hash
            .clone(),
        candid_sha256: evidence.materialization_result.candid_sha256.clone(),
    }
}

fn role_promotion_policy_decision(
    input: &RolePromotionInputV1,
    policy: &RolePromotionPolicyV1,
) -> RolePromotionPolicyDecisionV1 {
    let level_allowed = policy
        .allowed_promotion_levels
        .contains(&input.promotion_level);
    let claims = promotion_policy_claims_for_input(input);
    let policy_satisfied = level_allowed
        && (!policy
            .requirements
            .contains(&PromotionPolicyRequirementV1::SealedBytes)
            || input.promotion_level == PromotionArtifactLevelV1::SealedWasm)
        && (!policy
            .requirements
            .contains(&PromotionPolicyRequirementV1::ByteIdenticalWasm)
            || claims.contains(&PromotionPolicyClaimV1::ByteIdenticalWasm))
        && (!policy
            .requirements
            .contains(&PromotionPolicyRequirementV1::TargetConfigDigest)
            || claims.contains(&PromotionPolicyClaimV1::TargetConfigDigest));
    RolePromotionPolicyDecisionV1 {
        role: input.role.clone(),
        requested_promotion_level: input.promotion_level,
        allowed_promotion_levels: policy.allowed_promotion_levels.clone(),
        requirements: policy.requirements.clone(),
        claims,
        level_allowed,
        policy_satisfied,
    }
}

fn promotion_policy_claims_for_input(input: &RolePromotionInputV1) -> Vec<PromotionPolicyClaimV1> {
    let mut claims = Vec::with_capacity(2);
    if input.require_byte_identical_wasm {
        claims.push(PromotionPolicyClaimV1::ByteIdenticalWasm);
    }
    if input.require_target_embedded_config {
        claims.push(PromotionPolicyClaimV1::TargetConfigDigest);
    }
    claims
}

fn collect_policy_findings(
    decision: &RolePromotionPolicyDecisionV1,
    blockers: &mut Vec<SafetyFindingV1>,
) {
    if !decision.level_allowed {
        blockers.push(promotion_finding(
            "promotion_policy_level_not_allowed",
            format!(
                "role {} cannot use promotion level {:?}",
                decision.role, decision.requested_promotion_level
            ),
            SafetySeverityV1::HardFailure,
            &decision.role,
        ));
    }
    if decision
        .requirements
        .contains(&PromotionPolicyRequirementV1::SealedBytes)
        && decision.requested_promotion_level != PromotionArtifactLevelV1::SealedWasm
    {
        blockers.push(promotion_finding(
            "promotion_policy_must_use_sealed_bytes",
            format!("role {} must use sealed bytes", decision.role),
            SafetySeverityV1::HardFailure,
            &decision.role,
        ));
    }
    if decision
        .requirements
        .contains(&PromotionPolicyRequirementV1::ByteIdenticalWasm)
        && !decision
            .claims
            .contains(&PromotionPolicyClaimV1::ByteIdenticalWasm)
    {
        blockers.push(promotion_finding(
            "promotion_policy_byte_identity_required",
            format!("role {} requires byte-identical wasm", decision.role),
            SafetySeverityV1::HardFailure,
            &decision.role,
        ));
    }
    if decision
        .requirements
        .contains(&PromotionPolicyRequirementV1::TargetConfigDigest)
        && !decision
            .claims
            .contains(&PromotionPolicyClaimV1::TargetConfigDigest)
    {
        blockers.push(promotion_finding(
            "promotion_policy_target_config_digest_required",
            format!("role {} requires target config digest", decision.role),
            SafetySeverityV1::HardFailure,
            &decision.role,
        ));
    }
}

fn promotion_artifact_identity_groups(
    roles: &[RolePromotionArtifactIdentityV1],
) -> Vec<PromotionArtifactIdentityGroupV1> {
    let mut groups = BTreeMap::<String, PromotionArtifactIdentityGroupV1>::new();
    for role in roles {
        let identity_key = artifact_identity_key_for_role(role);
        let group = groups.entry(identity_key.clone()).or_insert_with(|| {
            PromotionArtifactIdentityGroupV1 {
                identity_key,
                identity_kind: role.identity_kind,
                roles: Vec::new(),
                source_kinds: Vec::new(),
                source_locators: Vec::new(),
                digest_pinned: role.digest_pinned,
                wasm_sha256: role.wasm_sha256.clone(),
                wasm_gz_sha256: role.wasm_gz_sha256.clone(),
                candid_sha256: role.candid_sha256.clone(),
                canonical_embedded_config_sha256: role.canonical_embedded_config_sha256.clone(),
            }
        });
        if !group.source_kinds.contains(&role.source_kind) {
            group.source_kinds.push(role.source_kind);
        }
        if let Some(locator) = &role.source_locator
            && !group.source_locators.contains(locator)
        {
            group.source_locators.push(locator.clone());
        }
        group.roles.push(role.role.clone());
    }
    groups.into_values().collect()
}

fn promotion_artifact_identity_summary(
    roles: &[RolePromotionArtifactIdentityV1],
    groups: &[PromotionArtifactIdentityGroupV1],
) -> PromotionArtifactIdentitySummaryV1 {
    PromotionArtifactIdentitySummaryV1 {
        role_count: roles.len(),
        identity_group_count: groups.len(),
        shared_identity_group_count: groups.iter().filter(|group| group.roles.len() > 1).count(),
        digest_pinned_role_count: roles.iter().filter(|role| role.digest_pinned).count(),
        source_build_role_count: roles
            .iter()
            .filter(|role| role.identity_kind == PromotionArtifactIdentityKindV1::SourceBuild)
            .count(),
        deferred_identity_role_count: roles
            .iter()
            .filter(|role| role.identity_kind == PromotionArtifactIdentityKindV1::Deferred)
            .count(),
    }
}

fn promotion_materialization_output_groups(
    roles: &[RolePromotionMaterializationIdentityV1],
) -> Vec<PromotionMaterializationOutputGroupV1> {
    let mut groups = BTreeMap::<String, PromotionMaterializationOutputGroupV1>::new();
    for role in roles {
        let output_identity_key = materialization_output_key_for_role(role);
        let group = groups
            .entry(output_identity_key.clone())
            .or_insert_with(|| PromotionMaterializationOutputGroupV1 {
                output_identity_key,
                roles: Vec::new(),
                wasm_sha256: role.wasm_sha256.clone(),
                wasm_gz_sha256: role.wasm_gz_sha256.clone(),
                installed_module_hash: role.installed_module_hash.clone(),
                candid_sha256: role.candid_sha256.clone(),
            });
        group.roles.push(role.role.clone());
    }
    groups.into_values().collect()
}

const fn promotion_artifact_identity_kind(
    promotion_level: PromotionArtifactLevelV1,
    source: &RoleArtifactSourceV1,
) -> PromotionArtifactIdentityKindV1 {
    if matches!(promotion_level, PromotionArtifactLevelV1::SourceBuild) {
        return PromotionArtifactIdentityKindV1::SourceBuild;
    }
    match (
        source.expected_wasm_sha256.is_some(),
        source.expected_wasm_gz_sha256.is_some(),
    ) {
        (true, true) => PromotionArtifactIdentityKindV1::SealedWasmAndCompressedWasm,
        (true, false) => PromotionArtifactIdentityKindV1::SealedWasm,
        (false, true) => PromotionArtifactIdentityKindV1::SealedCompressedWasm,
        (false, false) => PromotionArtifactIdentityKindV1::Deferred,
    }
}

fn artifact_identity_key_for_role(role: &RolePromotionArtifactIdentityV1) -> String {
    match role.identity_kind {
        PromotionArtifactIdentityKindV1::SealedWasm
        | PromotionArtifactIdentityKindV1::SealedCompressedWasm
        | PromotionArtifactIdentityKindV1::SealedWasmAndCompressedWasm => sealed_identity_key(
            role.wasm_sha256.as_deref(),
            role.wasm_gz_sha256.as_deref(),
            role.candid_sha256.as_deref(),
            role.canonical_embedded_config_sha256.as_deref(),
        ),
        PromotionArtifactIdentityKindV1::SourceBuild => format!(
            "source_build:source_kind={:?}:locator={}:candid={}:config={}",
            role.source_kind,
            optional_identity_part(role.source_locator.as_deref()),
            optional_identity_part(role.candid_sha256.as_deref()),
            optional_identity_part(role.canonical_embedded_config_sha256.as_deref())
        ),
        PromotionArtifactIdentityKindV1::Deferred => format!(
            "deferred:source_kind={:?}:locator={}",
            role.source_kind,
            optional_identity_part(role.source_locator.as_deref())
        ),
    }
}

fn artifact_identity_key_for_group(group: &PromotionArtifactIdentityGroupV1) -> String {
    match group.identity_kind {
        PromotionArtifactIdentityKindV1::SealedWasm
        | PromotionArtifactIdentityKindV1::SealedCompressedWasm
        | PromotionArtifactIdentityKindV1::SealedWasmAndCompressedWasm => sealed_identity_key(
            group.wasm_sha256.as_deref(),
            group.wasm_gz_sha256.as_deref(),
            group.candid_sha256.as_deref(),
            group.canonical_embedded_config_sha256.as_deref(),
        ),
        PromotionArtifactIdentityKindV1::SourceBuild => format!(
            "source_build:source_kind={}:locator={}:candid={}:config={}",
            source_kind_identity_part(single_group_source_kind(group)),
            optional_identity_part(single_group_source_locator(group)),
            optional_identity_part(group.candid_sha256.as_deref()),
            optional_identity_part(group.canonical_embedded_config_sha256.as_deref())
        ),
        PromotionArtifactIdentityKindV1::Deferred => format!(
            "deferred:source_kind={}:locator={}",
            source_kind_identity_part(single_group_source_kind(group)),
            optional_identity_part(single_group_source_locator(group))
        ),
    }
}

fn materialization_output_key_for_role(role: &RolePromotionMaterializationIdentityV1) -> String {
    materialization_output_key(
        &role.wasm_sha256,
        &role.wasm_gz_sha256,
        &role.installed_module_hash,
        &role.candid_sha256,
    )
}

fn materialization_output_key_for_group(group: &PromotionMaterializationOutputGroupV1) -> String {
    materialization_output_key(
        &group.wasm_sha256,
        &group.wasm_gz_sha256,
        &group.installed_module_hash,
        &group.candid_sha256,
    )
}

fn materialization_output_key(
    wasm_sha256: &str,
    wasm_gz_sha256: &str,
    installed_module_hash: &str,
    candid_sha256: &str,
) -> String {
    format!(
        "materialized:wasm={wasm_sha256}:wasm_gz={wasm_gz_sha256}:installed={installed_module_hash}:candid={candid_sha256}"
    )
}

fn source_kind_identity_part(kind: Option<RoleArtifactSourceKindV1>) -> String {
    kind.map_or_else(|| "not-recorded".to_string(), |kind| format!("{kind:?}"))
}

fn single_group_source_kind(
    group: &PromotionArtifactIdentityGroupV1,
) -> Option<RoleArtifactSourceKindV1> {
    group.source_kinds.first().copied()
}

fn single_group_source_locator(group: &PromotionArtifactIdentityGroupV1) -> Option<&str> {
    group.source_locators.first().map(String::as_str)
}

fn sealed_identity_key(
    wasm_sha256: Option<&str>,
    wasm_gz_sha256: Option<&str>,
    candid_sha256: Option<&str>,
    canonical_embedded_config_sha256: Option<&str>,
) -> String {
    format!(
        "sealed:wasm={}:wasm_gz={}:candid={}:config={}",
        optional_identity_part(wasm_sha256),
        optional_identity_part(wasm_gz_sha256),
        optional_identity_part(candid_sha256),
        optional_identity_part(canonical_embedded_config_sha256)
    )
}

const fn optional_identity_part(value: Option<&str>) -> &str {
    match value {
        Some(value) => value,
        None => "not-recorded",
    }
}

fn validate_role_artifact_identity(
    role: &RolePromotionArtifactIdentityV1,
) -> Result<(), PromotionArtifactIdentityReportError> {
    ensure_identity_report_field("role", &role.role)?;
    ensure_identity_optional_sha256("wasm_sha256", role.wasm_sha256.as_deref())?;
    ensure_identity_optional_sha256("wasm_gz_sha256", role.wasm_gz_sha256.as_deref())?;
    ensure_identity_optional_sha256("candid_sha256", role.candid_sha256.as_deref())?;
    ensure_identity_optional_sha256(
        "canonical_embedded_config_sha256",
        role.canonical_embedded_config_sha256.as_deref(),
    )?;
    Ok(())
}

fn validate_role_promotion_policy_decision(
    decision: &RolePromotionPolicyDecisionV1,
) -> Result<(), PromotionPolicyCheckError> {
    ensure_policy_field("role", &decision.role)?;
    if decision.allowed_promotion_levels.is_empty() {
        return Err(PromotionPolicyCheckError::EmptyAllowedLevels {
            role: decision.role.clone(),
        });
    }
    let mut seen = BTreeSet::new();
    for level in &decision.allowed_promotion_levels {
        if !seen.insert(*level) {
            return Err(PromotionPolicyCheckError::DuplicateAllowedLevel {
                role: decision.role.clone(),
                level: *level,
            });
        }
    }
    let mut seen_requirements = BTreeSet::new();
    for requirement in &decision.requirements {
        if !seen_requirements.insert(*requirement) {
            return Err(PromotionPolicyCheckError::DecisionMismatch {
                role: decision.role.clone(),
                field: "requirements",
            });
        }
    }
    let mut seen_claims = BTreeSet::new();
    for claim in &decision.claims {
        if !seen_claims.insert(*claim) {
            return Err(PromotionPolicyCheckError::DecisionMismatch {
                role: decision.role.clone(),
                field: "claims",
            });
        }
    }
    ensure_policy_decision(
        decision,
        "level_allowed",
        decision
            .allowed_promotion_levels
            .contains(&decision.requested_promotion_level)
            == decision.level_allowed,
    )?;
    ensure_policy_decision(
        decision,
        "policy_satisfied",
        promotion_policy_decision_satisfied(decision) == decision.policy_satisfied,
    )?;
    Ok(())
}

fn promotion_policy_decision_satisfied(decision: &RolePromotionPolicyDecisionV1) -> bool {
    decision.level_allowed
        && (!contains_policy_requirement(
            &decision.requirements,
            PromotionPolicyRequirementV1::SealedBytes,
        ) || matches!(
            decision.requested_promotion_level,
            PromotionArtifactLevelV1::SealedWasm
        ))
        && (!contains_policy_requirement(
            &decision.requirements,
            PromotionPolicyRequirementV1::ByteIdenticalWasm,
        ) || contains_policy_claim(&decision.claims, PromotionPolicyClaimV1::ByteIdenticalWasm))
        && (!contains_policy_requirement(
            &decision.requirements,
            PromotionPolicyRequirementV1::TargetConfigDigest,
        ) || contains_policy_claim(
            &decision.claims,
            PromotionPolicyClaimV1::TargetConfigDigest,
        ))
}

fn contains_policy_requirement(
    requirements: &[PromotionPolicyRequirementV1],
    needle: PromotionPolicyRequirementV1,
) -> bool {
    let mut index = 0;
    while index < requirements.len() {
        if requirements[index] as u8 == needle as u8 {
            return true;
        }
        index += 1;
    }
    false
}

fn contains_policy_claim(
    claims: &[PromotionPolicyClaimV1],
    needle: PromotionPolicyClaimV1,
) -> bool {
    let mut index = 0;
    while index < claims.len() {
        if claims[index] as u8 == needle as u8 {
            return true;
        }
        index += 1;
    }
    false
}

fn ensure_policy_decision(
    decision: &RolePromotionPolicyDecisionV1,
    field: &'static str,
    valid: bool,
) -> Result<(), PromotionPolicyCheckError> {
    if valid {
        Ok(())
    } else {
        Err(PromotionPolicyCheckError::DecisionMismatch {
            role: decision.role.clone(),
            field,
        })
    }
}

fn validate_artifact_identity_groups(
    roles: &[RolePromotionArtifactIdentityV1],
    groups: &[PromotionArtifactIdentityGroupV1],
) -> Result<(), PromotionArtifactIdentityReportError> {
    let role_names = roles
        .iter()
        .map(|role| role.role.as_str())
        .collect::<BTreeSet<_>>();
    let mut grouped_roles = BTreeSet::new();
    let mut group_keys = BTreeSet::new();
    for group in groups {
        validate_artifact_identity_group(group)?;
        if !group_keys.insert(group.identity_key.as_str()) {
            return Err(
                PromotionArtifactIdentityReportError::DuplicateIdentityGroup {
                    identity_key: group.identity_key.clone(),
                },
            );
        }
        if group.roles.is_empty() {
            return Err(PromotionArtifactIdentityReportError::EmptyIdentityGroup {
                identity_key: group.identity_key.clone(),
            });
        }
        for role in &group.roles {
            if !role_names.contains(role.as_str()) {
                return Err(PromotionArtifactIdentityReportError::UnknownGroupedRole {
                    role: role.clone(),
                });
            }
            if !grouped_roles.insert(role.as_str()) {
                return Err(PromotionArtifactIdentityReportError::DuplicateGroupedRole {
                    role: role.clone(),
                });
            }
            let role_identity = roles
                .iter()
                .find(|candidate| candidate.role == *role)
                .expect("known role should be present");
            let expected = artifact_identity_key_for_role(role_identity);
            if expected != group.identity_key {
                return Err(
                    PromotionArtifactIdentityReportError::IdentityGroupRoleMismatch {
                        role: role.clone(),
                        expected,
                        found: group.identity_key.clone(),
                    },
                );
            }
        }
    }
    for role in roles {
        if !grouped_roles.contains(role.role.as_str()) {
            return Err(PromotionArtifactIdentityReportError::MissingGroupedRole {
                role: role.role.clone(),
            });
        }
    }
    Ok(())
}

fn validate_artifact_identity_summary(
    report: &PromotionArtifactIdentityReportV1,
) -> Result<(), PromotionArtifactIdentityReportError> {
    let expected = promotion_artifact_identity_summary(&report.roles, &report.identity_groups);
    if report.summary.role_count != expected.role_count {
        return Err(PromotionArtifactIdentityReportError::SummaryMismatch {
            field: "role_count",
        });
    }
    if report.summary.identity_group_count != expected.identity_group_count {
        return Err(PromotionArtifactIdentityReportError::SummaryMismatch {
            field: "identity_group_count",
        });
    }
    if report.summary.shared_identity_group_count != expected.shared_identity_group_count {
        return Err(PromotionArtifactIdentityReportError::SummaryMismatch {
            field: "shared_identity_group_count",
        });
    }
    if report.summary.digest_pinned_role_count != expected.digest_pinned_role_count {
        return Err(PromotionArtifactIdentityReportError::SummaryMismatch {
            field: "digest_pinned_role_count",
        });
    }
    if report.summary.source_build_role_count != expected.source_build_role_count {
        return Err(PromotionArtifactIdentityReportError::SummaryMismatch {
            field: "source_build_role_count",
        });
    }
    if report.summary.deferred_identity_role_count != expected.deferred_identity_role_count {
        return Err(PromotionArtifactIdentityReportError::SummaryMismatch {
            field: "deferred_identity_role_count",
        });
    }
    Ok(())
}

fn validate_artifact_identity_group(
    group: &PromotionArtifactIdentityGroupV1,
) -> Result<(), PromotionArtifactIdentityReportError> {
    ensure_identity_report_field("identity_group.identity_key", &group.identity_key)?;
    if group.source_kinds.is_empty() {
        return Err(PromotionArtifactIdentityReportError::MissingRequiredField {
            field: "identity_group.source_kinds",
        });
    }
    ensure_identity_optional_sha256("identity_group.wasm_sha256", group.wasm_sha256.as_deref())?;
    ensure_identity_optional_sha256(
        "identity_group.wasm_gz_sha256",
        group.wasm_gz_sha256.as_deref(),
    )?;
    ensure_identity_optional_sha256(
        "identity_group.candid_sha256",
        group.candid_sha256.as_deref(),
    )?;
    ensure_identity_optional_sha256(
        "identity_group.canonical_embedded_config_sha256",
        group.canonical_embedded_config_sha256.as_deref(),
    )?;
    let expected = artifact_identity_key_for_group(group);
    if expected != group.identity_key {
        return Err(
            PromotionArtifactIdentityReportError::IdentityGroupKeyMismatch {
                expected,
                found: group.identity_key.clone(),
            },
        );
    }
    Ok(())
}

fn validate_materialization_output_groups(
    roles: &[RolePromotionMaterializationIdentityV1],
    groups: &[PromotionMaterializationOutputGroupV1],
) -> Result<(), PromotionMaterializationIdentityReportError> {
    let role_names = roles
        .iter()
        .map(|role| role.role.as_str())
        .collect::<BTreeSet<_>>();
    let mut grouped_roles = BTreeSet::new();
    let mut group_keys = BTreeSet::new();
    for group in groups {
        validate_materialization_output_group(group)?;
        if !group_keys.insert(group.output_identity_key.as_str()) {
            return Err(
                PromotionMaterializationIdentityReportError::DuplicateOutputGroup {
                    output_identity_key: group.output_identity_key.clone(),
                },
            );
        }
        if group.roles.is_empty() {
            return Err(
                PromotionMaterializationIdentityReportError::EmptyOutputGroup {
                    output_identity_key: group.output_identity_key.clone(),
                },
            );
        }
        for role in &group.roles {
            if !role_names.contains(role.as_str()) {
                return Err(
                    PromotionMaterializationIdentityReportError::UnknownGroupedRole {
                        role: role.clone(),
                    },
                );
            }
            if !grouped_roles.insert(role.as_str()) {
                return Err(
                    PromotionMaterializationIdentityReportError::DuplicateGroupedRole {
                        role: role.clone(),
                    },
                );
            }
            let role_identity = roles
                .iter()
                .find(|candidate| candidate.role == *role)
                .expect("known role should be present");
            let expected = materialization_output_key_for_role(role_identity);
            if expected != group.output_identity_key {
                return Err(
                    PromotionMaterializationIdentityReportError::OutputGroupRoleMismatch {
                        role: role.clone(),
                        expected,
                        found: group.output_identity_key.clone(),
                    },
                );
            }
        }
    }
    for role in roles {
        if !grouped_roles.contains(role.role.as_str()) {
            return Err(
                PromotionMaterializationIdentityReportError::MissingGroupedRole {
                    role: role.role.clone(),
                },
            );
        }
    }
    Ok(())
}

fn validate_materialization_output_group(
    group: &PromotionMaterializationOutputGroupV1,
) -> Result<(), PromotionMaterializationIdentityReportError> {
    ensure_materialization_report_field(
        "output_group.output_identity_key",
        &group.output_identity_key,
    )?;
    ensure_materialization_report_sha256("output_group.wasm_sha256", &group.wasm_sha256)?;
    ensure_materialization_report_sha256("output_group.wasm_gz_sha256", &group.wasm_gz_sha256)?;
    ensure_materialization_report_sha256(
        "output_group.installed_module_hash",
        &group.installed_module_hash,
    )?;
    ensure_materialization_report_sha256("output_group.candid_sha256", &group.candid_sha256)?;
    let expected = materialization_output_key_for_group(group);
    if expected != group.output_identity_key {
        return Err(
            PromotionMaterializationIdentityReportError::OutputGroupKeyMismatch {
                expected,
                found: group.output_identity_key.clone(),
            },
        );
    }
    Ok(())
}

fn validate_role_plan_transform(
    role: &RolePromotionPlanTransformV1,
    promoted_plan: &DeploymentPlanV1,
) -> Result<(), PromotionPlanTransformError> {
    ensure_transform_field("role", &role.role)?;
    let Some(promoted_role) = promoted_plan
        .role_artifacts
        .iter()
        .find(|artifact| artifact.role == role.role)
    else {
        return Err(PromotionPlanTransformError::PromotedRoleMissing {
            role: role.role.clone(),
        });
    };
    ensure_role_matches_promoted_artifact(role, promoted_role)?;
    ensure_role_transform_flags_are_consistent(role)?;
    validate_role_materialization_link(role, promoted_role)?;
    Ok(())
}

fn ensure_role_matches_promoted_artifact(
    role: &RolePromotionPlanTransformV1,
    promoted_role: &RoleArtifactV1,
) -> Result<(), PromotionPlanTransformError> {
    ensure_role_field_matches(
        role,
        "artifact_source_after",
        role.artifact_source_after == promoted_role.source,
    )?;
    ensure_role_field_matches(
        role,
        "wasm_sha256_after",
        role.wasm_sha256_after == promoted_role.wasm_sha256,
    )?;
    ensure_role_field_matches(
        role,
        "wasm_gz_sha256_after",
        role.wasm_gz_sha256_after == promoted_role.wasm_gz_sha256,
    )?;
    ensure_role_field_matches(
        role,
        "candid_sha256_after",
        role.candid_sha256_after == promoted_role.candid_sha256,
    )?;
    ensure_role_field_matches(
        role,
        "canonical_embedded_config_sha256_after",
        role.canonical_embedded_config_sha256_after
            == promoted_role.canonical_embedded_config_sha256,
    )
}

fn ensure_role_transform_flags_are_consistent(
    role: &RolePromotionPlanTransformV1,
) -> Result<(), PromotionPlanTransformError> {
    ensure_role_field_matches(
        role,
        "artifact_identity_changed",
        role.artifact_identity_changed == role_summary_artifact_identity_changed(role),
    )?;
    ensure_role_field_matches(
        role,
        "embedded_config_changed",
        role.embedded_config_changed
            == (role.canonical_embedded_config_sha256_before
                != role.canonical_embedded_config_sha256_after),
    )?;
    if role.target_materialization_preserved {
        ensure_role_field_matches(
            role,
            "target_materialization_preserved",
            role.promotion_level == PromotionArtifactLevelV1::SourceBuild
                && !role.artifact_identity_changed
                && !role.embedded_config_changed,
        )?;
    }
    Ok(())
}

fn validate_role_materialization_link(
    role: &RolePromotionPlanTransformV1,
    promoted_role: &RoleArtifactV1,
) -> Result<(), PromotionPlanTransformError> {
    let Some(link) = &role.source_build_materialization else {
        return Ok(());
    };
    ensure_role_field_matches(
        role,
        "source_build_materialization",
        role.promotion_level == PromotionArtifactLevelV1::SourceBuild,
    )?;
    ensure_role_field_matches(
        role,
        "source_build_materialization.role",
        link.role == role.role,
    )?;
    ensure_transform_field(
        "source_build_materialization.evidence_id",
        &link.evidence_id,
    )?;
    ensure_transform_field("source_build_materialization.recipe_id", &link.recipe_id)?;
    ensure_transform_field(
        "source_build_materialization.materialization_input_id",
        &link.materialization_input_id,
    )?;
    ensure_transform_field(
        "source_build_materialization.materialization_result_id",
        &link.materialization_result_id,
    )?;
    ensure_materialization_sha256(
        "source_build_materialization.materialization_input_digest",
        &link.materialization_input_digest,
    )?;
    ensure_materialization_sha256(
        "source_build_materialization.wasm_sha256",
        &link.wasm_sha256,
    )?;
    ensure_materialization_sha256(
        "source_build_materialization.wasm_gz_sha256",
        &link.wasm_gz_sha256,
    )?;
    ensure_materialization_sha256(
        "source_build_materialization.installed_module_hash",
        &link.installed_module_hash,
    )?;
    ensure_materialization_sha256(
        "source_build_materialization.candid_sha256",
        &link.candid_sha256,
    )?;
    ensure_role_field_matches(
        role,
        "source_build_materialization.wasm_sha256",
        promoted_role.wasm_sha256.as_deref() == Some(link.wasm_sha256.as_str()),
    )?;
    ensure_role_field_matches(
        role,
        "source_build_materialization.wasm_gz_sha256",
        promoted_role.wasm_gz_sha256.as_deref() == Some(link.wasm_gz_sha256.as_str()),
    )?;
    ensure_role_field_matches(
        role,
        "source_build_materialization.installed_module_hash",
        promoted_role.installed_module_hash.as_deref() == Some(link.installed_module_hash.as_str()),
    )?;
    ensure_role_field_matches(
        role,
        "source_build_materialization.candid_sha256",
        promoted_role.candid_sha256.as_deref() == Some(link.candid_sha256.as_str()),
    )
}

fn role_summary_artifact_identity_changed(role: &RolePromotionPlanTransformV1) -> bool {
    role.artifact_source_before != role.artifact_source_after
        || role.wasm_sha256_before != role.wasm_sha256_after
        || role.wasm_gz_sha256_before != role.wasm_gz_sha256_after
        || role.candid_sha256_before != role.candid_sha256_after
}

fn ensure_role_field_matches(
    role: &RolePromotionPlanTransformV1,
    field: &'static str,
    matches: bool,
) -> Result<(), PromotionPlanTransformError> {
    if matches {
        Ok(())
    } else {
        Err(PromotionPlanTransformError::RoleStateMismatch {
            role: role.role.clone(),
            field,
        })
    }
}

fn validate_role_readiness(role: &RolePromotionReadinessV1) -> Result<(), PromotionReadinessError> {
    ensure_readiness_field("role", &role.role)?;
    ensure_readiness_optional_sha256("source_wasm_sha256", role.source_wasm_sha256.as_deref())?;
    ensure_readiness_optional_sha256(
        "source_wasm_gz_sha256",
        role.source_wasm_gz_sha256.as_deref(),
    )?;
    ensure_readiness_optional_sha256("target_wasm_sha256", role.target_wasm_sha256.as_deref())?;
    ensure_readiness_optional_sha256(
        "target_wasm_gz_sha256",
        role.target_wasm_gz_sha256.as_deref(),
    )?;
    ensure_readiness_optional_sha256(
        "source_canonical_embedded_config_sha256",
        role.source_canonical_embedded_config_sha256.as_deref(),
    )?;
    ensure_readiness_optional_sha256(
        "target_canonical_embedded_config_sha256",
        role.target_canonical_embedded_config_sha256.as_deref(),
    )?;
    if role.restage_required != (role.target_store_has_artifact == Some(false)) {
        return Err(PromotionReadinessError::RestageStateMismatch {
            role: role.role.clone(),
        });
    }
    Ok(())
}

fn role_promotion_readiness(
    input: &RolePromotionInputV1,
    target_artifact: &RoleArtifactV1,
) -> RolePromotionReadinessV1 {
    let source_wasm_sha256 = input.source.expected_wasm_sha256.clone();
    let source_wasm_gz_sha256 = input.source.expected_wasm_gz_sha256.clone();
    let target_wasm_sha256 = target_artifact.wasm_sha256.clone();
    let target_wasm_gz_sha256 = target_artifact.wasm_gz_sha256.clone();
    let byte_identical_wasm =
        matching_optional_digest(source_wasm_sha256.as_ref(), target_wasm_sha256.as_ref()).or_else(
            || {
                matching_optional_digest(
                    source_wasm_gz_sha256.as_ref(),
                    target_wasm_gz_sha256.as_ref(),
                )
            },
        );
    let embedded_config_identical = matching_optional_digest(
        input
            .source
            .expected_canonical_embedded_config_sha256
            .as_ref(),
        target_artifact.canonical_embedded_config_sha256.as_ref(),
    );

    RolePromotionReadinessV1 {
        role: input.role.clone(),
        promotion_level: input.promotion_level,
        source_kind: input.source.kind,
        source_locator: input.source.locator.clone(),
        source_wasm_sha256,
        source_wasm_gz_sha256,
        target_wasm_sha256,
        target_wasm_gz_sha256,
        source_canonical_embedded_config_sha256: input
            .source
            .expected_canonical_embedded_config_sha256
            .clone(),
        target_canonical_embedded_config_sha256: target_artifact
            .canonical_embedded_config_sha256
            .clone(),
        byte_identical_wasm,
        embedded_config_identical,
        target_store_has_artifact: input.target_store_has_artifact,
        restage_required: input.target_store_has_artifact == Some(false),
    }
}

fn collect_role_findings(
    input: &RolePromotionInputV1,
    readiness: &RolePromotionReadinessV1,
    blockers: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    if let Err(err) = validate_role_artifact_source(&input.source) {
        blockers.push(promotion_finding(
            "promotion_artifact_source_invalid",
            err.to_string(),
            SafetySeverityV1::HardFailure,
            &input.role,
        ));
    }

    if input.role != input.source.role {
        blockers.push(promotion_finding(
            "promotion_source_role_mismatch",
            format!(
                "promotion input role {} does not match artifact source role {}",
                input.role, input.source.role
            ),
            SafetySeverityV1::HardFailure,
            &input.role,
        ));
    }

    if input.require_byte_identical_wasm && readiness.byte_identical_wasm != Some(true) {
        blockers.push(promotion_finding(
            "promotion_wasm_digest_mismatch",
            "promotion requires byte-identical wasm but source and target digests differ or are incomplete",
            SafetySeverityV1::HardFailure,
            &input.role,
        ));
    }

    if input.require_target_embedded_config
        && readiness
            .target_canonical_embedded_config_sha256
            .as_deref()
            .is_none_or(str::is_empty)
    {
        blockers.push(promotion_finding(
            "promotion_target_embedded_config_missing",
            "promotion requires target canonical embedded config but target plan has no digest",
            SafetySeverityV1::HardFailure,
            &input.role,
        ));
    }

    if input.promotion_level == PromotionArtifactLevelV1::SealedWasm
        && readiness.embedded_config_identical != Some(true)
    {
        blockers.push(promotion_finding(
            "promotion_sealed_wasm_embedded_config_mismatch",
            "sealed wasm promotion requires embedded config identity to be acceptable for the target",
            SafetySeverityV1::HardFailure,
            &input.role,
        ));
    }

    if readiness.restage_required {
        warnings.push(promotion_finding(
            "promotion_target_store_restage_required",
            "target artifact store does not already contain the artifact; restaging is required",
            SafetySeverityV1::Warning,
            &input.role,
        ));
    }
}

fn matching_optional_digest(left: Option<&String>, right: Option<&String>) -> Option<bool> {
    match (left.map(String::as_str), right.map(String::as_str)) {
        (Some(left), Some(right)) => Some(left == right),
        _ => None,
    }
}

fn promotion_finding(
    code: impl Into<String>,
    message: impl Into<String>,
    severity: SafetySeverityV1,
    role: &str,
) -> SafetyFindingV1 {
    SafetyFindingV1 {
        code: code.into(),
        message: message.into(),
        severity,
        subject: Some(role.to_string()),
    }
}

fn ensure_locator_requirement(
    source: &RoleArtifactSourceV1,
) -> Result<(), PromotionArtifactSourceError> {
    match source.kind {
        RoleArtifactSourceKindV1::CanonicalWasmStoreDefault => Ok(()),
        _ => ensure_option_field("locator", source.locator.as_deref()),
    }
}

const fn ensure_previous_receipt_requirement(
    source: &RoleArtifactSourceV1,
) -> Result<(), PromotionArtifactSourceError> {
    match (source.kind, source.previous_receipt_kind) {
        (RoleArtifactSourceKindV1::PreviousReceiptArtifact, Some(_)) => Ok(()),
        (RoleArtifactSourceKindV1::PreviousReceiptArtifact, None) => {
            Err(PromotionArtifactSourceError::MissingPreviousReceiptKind)
        }
        (_, Some(_)) => {
            Err(PromotionArtifactSourceError::UnexpectedPreviousReceiptKind { kind: source.kind })
        }
        (_, None) => Ok(()),
    }
}

const fn ensure_previous_receipt_lineage_digest_requirement(
    source: &RoleArtifactSourceV1,
) -> Result<(), PromotionArtifactSourceError> {
    match (source.kind, source.previous_receipt_lineage_digest.as_ref()) {
        (RoleArtifactSourceKindV1::PreviousReceiptArtifact, Some(_)) => Ok(()),
        (RoleArtifactSourceKindV1::PreviousReceiptArtifact, None) => {
            Err(PromotionArtifactSourceError::MissingPreviousReceiptLineageDigest)
        }
        (_, Some(_)) => Err(
            PromotionArtifactSourceError::UnexpectedPreviousReceiptLineageDigest {
                kind: source.kind,
            },
        ),
        (_, None) => Ok(()),
    }
}

const fn ensure_digest_requirement(
    source: &RoleArtifactSourceV1,
) -> Result<(), PromotionArtifactSourceError> {
    let has_digest =
        source.expected_wasm_sha256.is_some() || source.expected_wasm_gz_sha256.is_some();
    match source.kind {
        RoleArtifactSourceKindV1::LocalWasm if source.expected_wasm_sha256.is_none() => {
            Err(PromotionArtifactSourceError::MissingDigestPin { kind: source.kind })
        }
        RoleArtifactSourceKindV1::LocalWasmGz if source.expected_wasm_gz_sha256.is_none() => {
            Err(PromotionArtifactSourceError::MissingDigestPin { kind: source.kind })
        }
        RoleArtifactSourceKindV1::PublishedPackage
        | RoleArtifactSourceKindV1::PreviousReceiptArtifact
            if !has_digest =>
        {
            Err(PromotionArtifactSourceError::MissingDigestPin { kind: source.kind })
        }
        _ => Ok(()),
    }
}

fn ensure_option_field(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), PromotionArtifactSourceError> {
    match value {
        Some(value) => ensure_field(field, value),
        None => Err(PromotionArtifactSourceError::MissingRequiredField { field }),
    }
}

fn ensure_field(field: &'static str, value: &str) -> Result<(), PromotionArtifactSourceError> {
    if value.trim().is_empty() {
        return Err(PromotionArtifactSourceError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_optional_sha256(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), PromotionArtifactSourceError> {
    let Some(value) = value else {
        return Ok(());
    };
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionArtifactSourceError::InvalidSha256Digest { field })
    }
}

fn is_lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

const fn ensure_readiness_status_matches_blockers(
    readiness: &PromotionReadinessV1,
) -> Result<(), PromotionReadinessError> {
    match (readiness.status, readiness.blockers.is_empty()) {
        (PromotionReadinessStatusV1::Ready, false)
        | (PromotionReadinessStatusV1::Blocked, true) => {
            Err(PromotionReadinessError::StatusBlockerMismatch {
                status: readiness.status,
                blocker_count: readiness.blockers.len(),
            })
        }
        _ => Ok(()),
    }
}

fn ensure_unique_readiness_roles(
    roles: &[RolePromotionReadinessV1],
) -> Result<(), PromotionReadinessError> {
    let mut seen = std::collections::BTreeSet::new();
    for role in roles {
        if !seen.insert(role.role.as_str()) {
            return Err(PromotionReadinessError::DuplicateRole {
                role: role.role.clone(),
            });
        }
    }
    Ok(())
}

fn ensure_unique_transform_roles(
    roles: &[RolePromotionPlanTransformV1],
) -> Result<(), PromotionPlanTransformError> {
    let mut seen = std::collections::BTreeSet::new();
    for role in roles {
        if !seen.insert(role.role.as_str()) {
            return Err(PromotionPlanTransformError::DuplicateRole {
                role: role.role.clone(),
            });
        }
    }
    Ok(())
}

const fn ensure_policy_status_matches_blockers(
    check: &PromotionPolicyCheckV1,
) -> Result<(), PromotionPolicyCheckError> {
    match (check.status, check.blockers.is_empty()) {
        (PromotionReadinessStatusV1::Ready, false)
        | (PromotionReadinessStatusV1::Blocked, true) => {
            Err(PromotionPolicyCheckError::StatusBlockerMismatch {
                status: check.status,
                blocker_count: check.blockers.len(),
            })
        }
        _ => Ok(()),
    }
}

fn ensure_unique_policy_decision_roles(
    roles: &[RolePromotionPolicyDecisionV1],
) -> Result<(), PromotionPolicyCheckError> {
    let mut seen = BTreeSet::new();
    for role in roles {
        if !seen.insert(role.role.as_str()) {
            return Err(PromotionPolicyCheckError::DuplicateRole {
                role: role.role.clone(),
            });
        }
    }
    Ok(())
}

fn validate_policy_blockers(blockers: &[SafetyFindingV1]) -> Result<(), PromotionPolicyCheckError> {
    for blocker in blockers {
        ensure_policy_field("blocker.code", &blocker.code)?;
        ensure_policy_field("blocker.message", &blocker.message)?;
        if blocker.severity != SafetySeverityV1::HardFailure {
            return Err(PromotionPolicyCheckError::BlockerSeverityMismatch {
                severity: blocker.severity,
            });
        }
    }
    Ok(())
}

const fn ensure_identity_report_status_matches_blockers(
    report: &PromotionArtifactIdentityReportV1,
) -> Result<(), PromotionArtifactIdentityReportError> {
    match (report.status, report.blockers.is_empty()) {
        (PromotionReadinessStatusV1::Ready, false)
        | (PromotionReadinessStatusV1::Blocked, true) => Err(
            PromotionArtifactIdentityReportError::StatusBlockerMismatch {
                status: report.status,
                blocker_count: report.blockers.len(),
            },
        ),
        _ => Ok(()),
    }
}

fn ensure_unique_artifact_identity_roles(
    roles: &[RolePromotionArtifactIdentityV1],
) -> Result<(), PromotionArtifactIdentityReportError> {
    let mut seen = std::collections::BTreeSet::new();
    for role in roles {
        if !seen.insert(role.role.as_str()) {
            return Err(PromotionArtifactIdentityReportError::DuplicateRole {
                role: role.role.clone(),
            });
        }
    }
    Ok(())
}

fn validate_identity_report_blockers(
    blockers: &[SafetyFindingV1],
) -> Result<(), PromotionArtifactIdentityReportError> {
    for blocker in blockers {
        ensure_identity_report_field("blocker.code", &blocker.code)?;
        ensure_identity_report_field("blocker.message", &blocker.message)?;
        if blocker.severity != SafetySeverityV1::HardFailure {
            return Err(
                PromotionArtifactIdentityReportError::BlockerSeverityMismatch {
                    severity: blocker.severity,
                },
            );
        }
    }
    Ok(())
}

const fn ensure_wasm_store_identity_status_matches_blockers(
    report: &PromotionWasmStoreIdentityReportV1,
) -> Result<(), PromotionWasmStoreIdentityReportError> {
    match (report.status, report.blockers.is_empty()) {
        (PromotionReadinessStatusV1::Ready, false)
        | (PromotionReadinessStatusV1::Blocked, true) => Err(
            PromotionWasmStoreIdentityReportError::StatusBlockerMismatch {
                status: report.status,
                blocker_count: report.blockers.len(),
            },
        ),
        _ => Ok(()),
    }
}

fn ensure_unique_wasm_store_identity_roles(
    roles: &[RolePromotionWasmStoreIdentityV1],
) -> Result<(), PromotionWasmStoreIdentityReportError> {
    let mut seen = BTreeSet::new();
    for role in roles {
        if !seen.insert(role.role.as_str()) {
            return Err(PromotionWasmStoreIdentityReportError::DuplicateRole {
                role: role.role.clone(),
            });
        }
    }
    Ok(())
}

fn ensure_wasm_store_identity_staging_receipts(
    receipts: &[StagingReceiptV1],
) -> Result<(), PromotionWasmStoreIdentityReportError> {
    for receipt in receipts {
        if receipt.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
            return Err(
                PromotionWasmStoreIdentityReportError::StagingReceiptSchemaVersionMismatch {
                    role: receipt.role.clone(),
                    expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                    found: receipt.schema_version,
                },
            );
        }
        ensure_wasm_store_identity_report_field("role", &receipt.role)?;
        ensure_wasm_store_identity_report_field("artifact_identity", &receipt.artifact_identity)?;
    }
    Ok(())
}

fn validate_role_wasm_store_identity(
    role: &RolePromotionWasmStoreIdentityV1,
) -> Result<(), PromotionWasmStoreIdentityReportError> {
    ensure_wasm_store_identity_report_field("role", &role.role)?;
    ensure_wasm_store_identity_report_field("artifact_identity", &role.artifact_identity)?;
    if let Some(locator) = &role.wasm_store_locator {
        ensure_wasm_store_identity_report_field("wasm_store_locator", locator)?;
    }
    for chunk_hash in &role.prepared_chunk_hashes {
        ensure_wasm_store_identity_report_field("prepared_chunk_hash", chunk_hash)?;
    }
    Ok(())
}

fn validate_wasm_store_identity_blockers(
    blockers: &[SafetyFindingV1],
) -> Result<(), PromotionWasmStoreIdentityReportError> {
    for blocker in blockers {
        ensure_wasm_store_identity_report_field("blocker.code", &blocker.code)?;
        ensure_wasm_store_identity_report_field("blocker.message", &blocker.message)?;
        if blocker.severity != SafetySeverityV1::HardFailure {
            return Err(
                PromotionWasmStoreIdentityReportError::BlockerSeverityMismatch {
                    severity: blocker.severity,
                },
            );
        }
    }
    Ok(())
}

const fn ensure_materialization_report_status_matches_blockers(
    report: &PromotionMaterializationIdentityReportV1,
) -> Result<(), PromotionMaterializationIdentityReportError> {
    match (report.status, report.blockers.is_empty()) {
        (PromotionReadinessStatusV1::Ready, false)
        | (PromotionReadinessStatusV1::Blocked, true) => Err(
            PromotionMaterializationIdentityReportError::StatusBlockerMismatch {
                status: report.status,
                blocker_count: report.blockers.len(),
            },
        ),
        _ => Ok(()),
    }
}

fn ensure_unique_materialization_report_roles(
    roles: &[RolePromotionMaterializationIdentityV1],
) -> Result<(), PromotionMaterializationIdentityReportError> {
    let mut seen_roles = BTreeSet::new();
    let mut seen_evidence = BTreeSet::new();
    for role in roles {
        if !seen_roles.insert(role.role.as_str()) {
            return Err(PromotionMaterializationIdentityReportError::DuplicateRole {
                role: role.role.clone(),
            });
        }
        if !seen_evidence.insert(role.evidence_id.as_str()) {
            return Err(
                PromotionMaterializationIdentityReportError::DuplicateEvidence {
                    evidence_id: role.evidence_id.clone(),
                },
            );
        }
    }
    Ok(())
}

fn validate_role_materialization_identity(
    role: &RolePromotionMaterializationIdentityV1,
) -> Result<(), PromotionMaterializationIdentityReportError> {
    ensure_materialization_report_field("role", &role.role)?;
    ensure_materialization_report_field("evidence_id", &role.evidence_id)?;
    ensure_materialization_report_field("recipe_id", &role.recipe_id)?;
    ensure_materialization_report_field(
        "materialization_input_id",
        &role.materialization_input_id,
    )?;
    ensure_materialization_report_field(
        "materialization_result_id",
        &role.materialization_result_id,
    )?;
    ensure_materialization_report_sha256(
        "materialization_input_digest",
        &role.materialization_input_digest,
    )?;
    ensure_materialization_report_sha256(
        "canonical_embedded_config_sha256",
        &role.canonical_embedded_config_sha256,
    )?;
    ensure_materialization_report_field("network", &role.network)?;
    ensure_materialization_report_field("root_trust_anchor", &role.root_trust_anchor)?;
    ensure_materialization_report_field("runtime_variant", &role.runtime_variant)?;
    ensure_materialization_report_sha256("wasm_sha256", &role.wasm_sha256)?;
    ensure_materialization_report_sha256("wasm_gz_sha256", &role.wasm_gz_sha256)?;
    ensure_materialization_report_sha256("installed_module_hash", &role.installed_module_hash)?;
    ensure_materialization_report_sha256("candid_sha256", &role.candid_sha256)?;
    Ok(())
}

fn validate_materialization_report_blockers(
    blockers: &[SafetyFindingV1],
) -> Result<(), PromotionMaterializationIdentityReportError> {
    for blocker in blockers {
        ensure_materialization_report_field("blocker.code", &blocker.code)?;
        ensure_materialization_report_field("blocker.message", &blocker.message)?;
        if blocker.severity != SafetySeverityV1::HardFailure {
            return Err(
                PromotionMaterializationIdentityReportError::BlockerSeverityMismatch {
                    severity: blocker.severity,
                },
            );
        }
    }
    Ok(())
}

const fn ensure_provenance_report_status_matches_blockers(
    report: &ArtifactPromotionProvenanceReportV1,
) -> Result<(), ArtifactPromotionProvenanceReportError> {
    match (report.status, report.blockers.is_empty()) {
        (PromotionReadinessStatusV1::Ready, false)
        | (PromotionReadinessStatusV1::Blocked, true) => Err(
            ArtifactPromotionProvenanceReportError::StatusBlockerMismatch {
                status: report.status,
                blocker_count: report.blockers.len(),
            },
        ),
        _ => Ok(()),
    }
}

fn ensure_unique_provenance_roles(
    roles: &[RolePromotionProvenanceV1],
) -> Result<(), ArtifactPromotionProvenanceReportError> {
    let mut seen = BTreeSet::new();
    for role in roles {
        if !seen.insert(role.role.as_str()) {
            return Err(ArtifactPromotionProvenanceReportError::DuplicateRole {
                role: role.role.clone(),
            });
        }
    }
    Ok(())
}

fn validate_role_promotion_provenance(
    role: &RolePromotionProvenanceV1,
) -> Result<(), ArtifactPromotionProvenanceReportError> {
    ensure_provenance_report_field("role", &role.role)?;
    if let Some(evidence_id) = &role.materialization_evidence_id {
        ensure_provenance_report_field("materialization_evidence_id", evidence_id)?;
    }
    if let Some(locator) = &role.wasm_store_locator {
        ensure_provenance_report_field("wasm_store_locator", locator)?;
    }
    Ok(())
}

fn validate_provenance_report_blockers(
    blockers: &[SafetyFindingV1],
) -> Result<(), ArtifactPromotionProvenanceReportError> {
    for blocker in blockers {
        ensure_provenance_report_field("blocker.code", &blocker.code)?;
        ensure_provenance_report_field("blocker.message", &blocker.message)?;
        if blocker.severity != SafetySeverityV1::HardFailure {
            return Err(
                ArtifactPromotionProvenanceReportError::BlockerSeverityMismatch {
                    severity: blocker.severity,
                },
            );
        }
    }
    Ok(())
}

fn ensure_execution_receipt_linkage(
    receipt: &ArtifactPromotionExecutionReceiptV1,
) -> Result<(), ArtifactPromotionExecutionReceiptError> {
    if receipt.deployment_receipt.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            ArtifactPromotionExecutionReceiptError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: receipt.deployment_receipt.schema_version,
            },
        );
    }
    if receipt.deployment_receipt.plan_id != receipt.promoted_plan_id {
        return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "deployment_receipt.plan_id",
        });
    }
    if receipt.deployment_receipt.operation_id != receipt.operation_id {
        return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "operation_id",
        });
    }
    if receipt.deployment_receipt.operation_status != receipt.operation_status {
        return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "operation_status",
        });
    }
    if receipt.deployment_receipt.command_result != receipt.command_result {
        return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "command_result",
        });
    }
    if receipt.deployment_receipt.started_at != receipt.started_at {
        return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "started_at",
        });
    }
    if receipt.deployment_receipt.finished_at != receipt.finished_at {
        return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "finished_at",
        });
    }
    ensure_execution_receipt_roles_match_deployment_receipt(
        &receipt.roles,
        &receipt.deployment_receipt,
    )?;
    ensure_unique_execution_receipt_roles(&receipt.roles)
}

const fn ensure_execution_receipt_provenance_ready(
    status: PromotionReadinessStatusV1,
) -> Result<(), ArtifactPromotionExecutionReceiptError> {
    if matches!(status, PromotionReadinessStatusV1::Ready) {
        Ok(())
    } else {
        Err(ArtifactPromotionExecutionReceiptError::ProvenanceNotReady { status })
    }
}

fn ensure_execution_receipt_roles_match_deployment_receipt(
    roles: &[RolePromotionExecutionReceiptV1],
    deployment_receipt: &DeploymentReceiptV1,
) -> Result<(), ArtifactPromotionExecutionReceiptError> {
    let promotion_roles = roles
        .iter()
        .map(|role| role.role.as_str())
        .collect::<BTreeSet<_>>();
    let deployment_roles = deployment_receipt
        .role_phase_receipts
        .iter()
        .map(|receipt| receipt.role.as_str())
        .collect::<BTreeSet<_>>();
    for role in &promotion_roles {
        if !deployment_roles.contains(role) {
            return Err(
                ArtifactPromotionExecutionReceiptError::MissingDeploymentRole {
                    role: (*role).to_string(),
                },
            );
        }
    }
    for role in &deployment_roles {
        if !promotion_roles.contains(role) {
            return Err(
                ArtifactPromotionExecutionReceiptError::UnknownDeploymentRole {
                    role: (*role).to_string(),
                },
            );
        }
    }
    for role in roles {
        let role_receipt = deployment_receipt
            .role_phase_receipts
            .iter()
            .rev()
            .find(|receipt| receipt.role == role.role);
        if role.role_phase_result != role_receipt.map(|receipt| receipt.result) {
            return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
                field: "role_phase_result",
            });
        }
        if role.artifact_digest != role_receipt.and_then(|receipt| receipt.artifact_digest.clone())
        {
            return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
                field: "artifact_digest",
            });
        }
        if role.observed_module_hash_after
            != role_receipt.and_then(|receipt| receipt.observed_module_hash_after.clone())
        {
            return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
                field: "observed_module_hash_after",
            });
        }
        if role.canonical_embedded_config_sha256
            != role_receipt.and_then(|receipt| receipt.canonical_embedded_config_sha256.clone())
        {
            return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
                field: "canonical_embedded_config_sha256",
            });
        }
    }
    Ok(())
}

fn ensure_unique_execution_receipt_roles(
    roles: &[RolePromotionExecutionReceiptV1],
) -> Result<(), ArtifactPromotionExecutionReceiptError> {
    let mut seen = BTreeSet::new();
    for role in roles {
        ensure_execution_receipt_field("role", &role.role)?;
        if !seen.insert(role.role.as_str()) {
            return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch { field: "roles" });
        }
        if let Some(evidence_id) = &role.materialization_evidence_id {
            ensure_execution_receipt_field("materialization_evidence_id", evidence_id)?;
        }
        if let Some(locator) = &role.wasm_store_locator {
            ensure_execution_receipt_field("wasm_store_locator", locator)?;
        }
        if let Some(digest) = &role.artifact_digest {
            ensure_execution_receipt_field("artifact_digest", digest)?;
        }
        if let Some(hash) = &role.observed_module_hash_after {
            ensure_execution_receipt_field("observed_module_hash_after", hash)?;
        }
        if let Some(digest) = &role.canonical_embedded_config_sha256 {
            ensure_execution_receipt_field("canonical_embedded_config_sha256", digest)?;
        }
    }
    Ok(())
}

fn validate_readiness_findings(
    field: &'static str,
    findings: &[SafetyFindingV1],
    expected_severity: SafetySeverityV1,
) -> Result<(), PromotionReadinessError> {
    for finding in findings {
        ensure_readiness_field("finding.code", &finding.code)?;
        ensure_readiness_field("finding.message", &finding.message)?;
        if finding.severity != expected_severity {
            return Err(PromotionReadinessError::FindingSeverityMismatch {
                field,
                severity: finding.severity,
            });
        }
    }
    Ok(())
}

fn ensure_policy_field(field: &'static str, value: &str) -> Result<(), PromotionPolicyCheckError> {
    if value.trim().is_empty() {
        return Err(PromotionPolicyCheckError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_identity_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionArtifactIdentityReportError> {
    if value.trim().is_empty() {
        return Err(PromotionArtifactIdentityReportError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_identity_optional_sha256(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), PromotionArtifactIdentityReportError> {
    let Some(value) = value else {
        return Ok(());
    };
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionArtifactIdentityReportError::InvalidSha256Digest { field })
    }
}

fn ensure_wasm_store_identity_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionWasmStoreIdentityReportError> {
    if value.trim().is_empty() {
        return Err(PromotionWasmStoreIdentityReportError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_materialization_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionMaterializationIdentityReportError> {
    if value.trim().is_empty() {
        return Err(PromotionMaterializationIdentityReportError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_provenance_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), ArtifactPromotionProvenanceReportError> {
    if value.trim().is_empty() {
        return Err(ArtifactPromotionProvenanceReportError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_execution_receipt_field(
    field: &'static str,
    value: &str,
) -> Result<(), ArtifactPromotionExecutionReceiptError> {
    if value.trim().is_empty() {
        return Err(ArtifactPromotionExecutionReceiptError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_materialization_report_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionMaterializationIdentityReportError> {
    ensure_materialization_report_field(field, value)?;
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(
            PromotionMaterializationIdentityReportError::Materialization(
                PromotionMaterializationIdentityError::InvalidSha256Digest { field },
            ),
        )
    }
}

fn ensure_materialization_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionMaterializationIdentityError> {
    if value.trim().is_empty() {
        return Err(PromotionMaterializationIdentityError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_materialization_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionMaterializationIdentityError> {
    ensure_materialization_field(field, value)?;
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionMaterializationIdentityError::InvalidSha256Digest { field })
    }
}

const fn ensure_materialization_link(
    field: &'static str,
    valid: bool,
) -> Result<(), PromotionMaterializationIdentityError> {
    if valid {
        Ok(())
    } else {
        Err(PromotionMaterializationIdentityError::LinkageMismatch { field })
    }
}

fn ensure_readiness_field(field: &'static str, value: &str) -> Result<(), PromotionReadinessError> {
    if value.trim().is_empty() {
        return Err(PromotionReadinessError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_readiness_optional_sha256(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), PromotionReadinessError> {
    let Some(value) = value else {
        return Ok(());
    };
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionReadinessError::InvalidSha256Digest { field })
    }
}

fn ensure_transform_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionPlanTransformError> {
    if value.trim().is_empty() {
        return Err(PromotionPlanTransformError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_evidence_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionPlanTransformEvidenceError> {
    if value.trim().is_empty() {
        return Err(PromotionPlanTransformEvidenceError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_artifact_promotion_plan_field(
    field: &'static str,
    value: &str,
) -> Result<(), ArtifactPromotionPlanError> {
    if value.trim().is_empty() {
        return Err(ArtifactPromotionPlanError::MissingRequiredField { field });
    }
    Ok(())
}

const fn ensure_artifact_promotion_status_matches_blockers(
    plan: &ArtifactPromotionPlanV1,
) -> Result<(), ArtifactPromotionPlanError> {
    match (plan.status, plan.blockers.is_empty()) {
        (PromotionReadinessStatusV1::Ready, false)
        | (PromotionReadinessStatusV1::Blocked, true) => {
            Err(ArtifactPromotionPlanError::StatusBlockerMismatch {
                status: plan.status,
                blocker_count: plan.blockers.len(),
            })
        }
        _ => Ok(()),
    }
}

fn ensure_artifact_promotion_plan_linkage(
    plan: &ArtifactPromotionPlanV1,
) -> Result<(), ArtifactPromotionPlanError> {
    let expected_blockers =
        artifact_promotion_plan_blockers(&plan.readiness, &plan.artifact_identity_report);
    if expected_blockers != plan.blockers {
        return Err(ArtifactPromotionPlanError::LinkageMismatch { field: "blockers" });
    }
    if plan.readiness.target_plan_id != plan.target_plan_id {
        return Err(ArtifactPromotionPlanError::LinkageMismatch {
            field: "readiness.target_plan_id",
        });
    }
    if plan.transform.target_plan_id != plan.target_plan_id {
        return Err(ArtifactPromotionPlanError::LinkageMismatch {
            field: "transform.target_plan_id",
        });
    }
    if plan.transform.promoted_plan_id != plan.promoted_plan_id {
        return Err(ArtifactPromotionPlanError::LinkageMismatch {
            field: "transform.promoted_plan_id",
        });
    }
    if plan.transform.promotion_plan_lineage_digest != plan.promotion_plan_lineage_digest {
        return Err(ArtifactPromotionPlanError::LinkageMismatch {
            field: "promotion_plan_lineage_digest",
        });
    }
    Ok(())
}

fn ensure_target_execution_lineage_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionTargetExecutionLineageError> {
    if value.trim().is_empty() {
        return Err(PromotionTargetExecutionLineageError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_target_execution_lineage_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionTargetExecutionLineageError> {
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionTargetExecutionLineageError::InvalidSha256Digest { field })
    }
}
