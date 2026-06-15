use super::super::super::LifecycleVerificationRequirementV1;

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
