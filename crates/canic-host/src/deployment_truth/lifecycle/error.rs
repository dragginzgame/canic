use super::super::LifecycleVerificationRequirementV1;

///
/// ExternalUpgradeReceiptError
///
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum ExternalUpgradeReceiptError {
    #[error("external upgrade receipt schema version {actual} does not match expected {expected}")]
    SchemaVersionMismatch { expected: u32, actual: u32 },
    #[error("external upgrade receipt field `{field}` is required")]
    MissingRequiredField { field: &'static str },
    #[error("external upgrade receipt field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },
    #[error("external upgrade receipt field `{field}` does not match proposal source")]
    SourceMismatch { field: &'static str },
    #[error("external upgrade receipt verification result does not match observations")]
    VerificationMismatch,
    #[error("external upgrade receipt refused consent cannot be verified")]
    RefusedConsentVerified,
}

///
/// ExternalUpgradeConsentEvidenceError
///
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum ExternalUpgradeConsentEvidenceError {
    #[error(
        "external upgrade consent evidence schema version {actual} does not match expected {expected}"
    )]
    SchemaVersionMismatch { expected: u32, actual: u32 },
    #[error("external upgrade consent evidence field `{field}` is required")]
    MissingRequiredField { field: &'static str },
    #[error("external upgrade consent evidence field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },
    #[error("external upgrade consent evidence field `{field}` no longer matches source receipt")]
    SourceMismatch { field: &'static str },
    #[error(transparent)]
    Receipt(#[from] ExternalUpgradeReceiptError),
}

///
/// ExternalUpgradeVerificationReportError
///
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum ExternalUpgradeVerificationReportError {
    #[error(
        "external upgrade verification report schema version {actual} does not match expected {expected}"
    )]
    SchemaVersionMismatch { expected: u32, actual: u32 },
    #[error("external upgrade verification report field `{field}` is required")]
    MissingRequiredField { field: &'static str },
    #[error("external upgrade verification report field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },
    #[error("external upgrade verification report field `{field}` does not match source evidence")]
    SourceMismatch { field: &'static str },
    #[error(transparent)]
    Receipt(#[from] ExternalUpgradeReceiptError),
}

///
/// ExternalUpgradeVerificationPolicyError
///
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum ExternalUpgradeVerificationPolicyError {
    #[error(
        "external upgrade verification policy schema version {actual} does not match expected {expected}"
    )]
    SchemaVersionMismatch { expected: u32, actual: u32 },
    #[error("external upgrade verification policy field `{field}` is required")]
    MissingRequiredField { field: &'static str },
    #[error("external upgrade verification policy field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },
    #[error("external upgrade verification policy field `{field}` does not match proposal source")]
    SourceMismatch { field: &'static str },
}

///
/// ExternalUpgradeVerificationCheckError
///
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum ExternalUpgradeVerificationCheckError {
    #[error(
        "external upgrade verification check schema version {actual} does not match expected {expected}"
    )]
    SchemaVersionMismatch { expected: u32, actual: u32 },
    #[error("external upgrade verification check field `{field}` is required")]
    MissingRequiredField { field: &'static str },
    #[error("external upgrade verification check field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },
    #[error("external upgrade verification check field `{field}` does not match policy source")]
    SourceMismatch { field: &'static str },
    #[error("external upgrade verification check contains duplicate requirement `{requirement:?}`")]
    DuplicateRequirement {
        requirement: LifecycleVerificationRequirementV1,
    },
    #[error(
        "external upgrade verification check requirement `{requirement:?}` has invalid satisfaction state"
    )]
    RequirementStatusMismatch {
        requirement: LifecycleVerificationRequirementV1,
    },
}

///
/// ExternalUpgradeCompletionReportError
///
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum ExternalUpgradeCompletionReportError {
    #[error(
        "external upgrade completion report schema version {actual} does not match expected {expected}"
    )]
    SchemaVersionMismatch { expected: u32, actual: u32 },
    #[error("external upgrade completion report field `{field}` is required")]
    MissingRequiredField { field: &'static str },
    #[error("external upgrade completion report field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },
    #[error("external upgrade completion report field `{field}` does not match source evidence")]
    SourceMismatch { field: &'static str },
    #[error(transparent)]
    Proposal(#[from] ExternalUpgradeProposalReportError),
    #[error(transparent)]
    ConsentEvidence(#[from] ExternalUpgradeConsentEvidenceError),
    #[error(transparent)]
    VerificationCheck(#[from] ExternalUpgradeVerificationCheckError),
}

///
/// LifecycleAuthorityReportError
///
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum LifecycleAuthorityReportError {
    #[error(
        "lifecycle authority report schema version {actual} does not match expected {expected}"
    )]
    SchemaVersionMismatch { expected: u32, actual: u32 },
    #[error("lifecycle authority report field `{field}` is required")]
    MissingRequiredField { field: &'static str },
    #[error("lifecycle authority report field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },
    #[error("lifecycle authority report contains duplicate subject `{subject}`")]
    DuplicateSubject { subject: String },
    #[error("lifecycle authority report counters do not match authority rows")]
    CountMismatch,
}

///
/// ExternalLifecyclePlanError
///
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum ExternalLifecyclePlanError {
    #[error("external lifecycle plan schema version {actual} does not match expected {expected}")]
    SchemaVersionMismatch { expected: u32, actual: u32 },
    #[error("external lifecycle plan field `{field}` is required")]
    MissingRequiredField { field: &'static str },
    #[error("external lifecycle plan field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },
    #[error("external lifecycle plan field `{field}` does not match deployment truth source")]
    SourceMismatch { field: &'static str },
    #[error("external lifecycle plan status does not match role partitioning")]
    StatusMismatch,
    #[error("external lifecycle plan contains duplicate subject `{subject}`")]
    DuplicateSubject { subject: String },
}

///
/// ExternalUpgradeProposalReportError
///
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum ExternalUpgradeProposalReportError {
    #[error(
        "external upgrade proposal report schema version {actual} does not match expected {expected}"
    )]
    SchemaVersionMismatch { expected: u32, actual: u32 },
    #[error("external upgrade proposal report field `{field}` is required")]
    MissingRequiredField { field: &'static str },
    #[error("external upgrade proposal report field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },
    #[error("external upgrade proposal report field `{field}` does not match lifecycle source")]
    SourceMismatch { field: &'static str },
    #[error(
        "external upgrade proposal report contains proposal for directly controlled row `{subject}`"
    )]
    DirectLifecycleProposal { subject: String },
    #[error("external upgrade proposal report contains duplicate subject `{subject}`")]
    DuplicateSubject { subject: String },
}

///
/// ExternalLifecyclePendingReportError
///
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum ExternalLifecyclePendingReportError {
    #[error(
        "external lifecycle pending report schema version {actual} does not match expected {expected}"
    )]
    SchemaVersionMismatch { expected: u32, actual: u32 },
    #[error("external lifecycle pending report field `{field}` is required")]
    MissingRequiredField { field: &'static str },
    #[error("external lifecycle pending report field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },
    #[error("external lifecycle pending report field `{field}` does not match lifecycle source")]
    SourceMismatch { field: &'static str },
    #[error("external lifecycle pending report counters do not match action rows")]
    CountMismatch,
    #[error("external lifecycle pending report contains duplicate subject `{subject}`")]
    DuplicateSubject { subject: String },
}

///
/// ExternalLifecycleCheckError
///
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum ExternalLifecycleCheckError {
    #[error("external lifecycle check schema version {actual} does not match expected {expected}")]
    SchemaVersionMismatch { expected: u32, actual: u32 },
    #[error("external lifecycle check field `{field}` is required")]
    MissingRequiredField { field: &'static str },
    #[error("external lifecycle check field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },
    #[error("external lifecycle check field `{field}` does not match lifecycle source")]
    SourceMismatch { field: &'static str },
    #[error("external lifecycle check counters do not match source reports")]
    CountMismatch,
}

///
/// ExternalLifecycleHandoffError
///
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum ExternalLifecycleHandoffError {
    #[error(
        "external lifecycle handoff schema version {actual} does not match expected {expected}"
    )]
    SchemaVersionMismatch { expected: u32, actual: u32 },
    #[error("external lifecycle handoff field `{field}` is required")]
    MissingRequiredField { field: &'static str },
    #[error("external lifecycle handoff field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },
    #[error("external lifecycle handoff field `{field}` does not match lifecycle source")]
    SourceMismatch { field: &'static str },
    #[error("external lifecycle handoff contains duplicate subject `{subject}`")]
    DuplicateSubject { subject: String },
}

///
/// CriticalExternalFixReportError
///
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum CriticalExternalFixReportError {
    #[error(
        "critical external fix report schema version {actual} does not match expected {expected}"
    )]
    SchemaVersionMismatch { expected: u32, actual: u32 },
    #[error("critical external fix report field `{field}` is required")]
    MissingRequiredField { field: &'static str },
    #[error("critical external fix report field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },
    #[error("critical external fix report field `{field}` does not match lifecycle source")]
    SourceMismatch { field: &'static str },
}
