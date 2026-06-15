use thiserror::Error as ThisError;

///
/// DeploymentRootVerificationReportError
///
#[derive(Debug, Eq, PartialEq, ThisError)]
pub enum DeploymentRootVerificationReportError {
    #[error(
        "deployment root verification report schema version {actual} does not match expected {expected}"
    )]
    SchemaVersionMismatch { expected: u32, actual: u32 },

    #[error("deployment root verification report field `{field}` is required")]
    MissingRequiredField { field: &'static str },

    #[error("deployment root verification report field `{field}` must be lowercase SHA-256 hex")]
    InvalidSha256Digest { field: &'static str },

    #[error("deployment root verification report field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },

    #[error("deployment root verification report check `{check}` is inconsistent")]
    CheckMismatch { check: String },

    #[error("deployment root verification report status is inconsistent")]
    StatusMismatch,
}

///
/// DeploymentRootVerificationReceiptError
///
#[derive(Debug, Eq, PartialEq, ThisError)]
pub enum DeploymentRootVerificationReceiptError {
    #[error(
        "deployment root verification receipt schema version {actual} does not match expected {expected}"
    )]
    SchemaVersionMismatch { expected: u32, actual: u32 },

    #[error("deployment root verification receipt field `{field}` is required")]
    MissingRequiredField { field: &'static str },

    #[error("deployment root verification receipt field `{field}` must be lowercase SHA-256 hex")]
    InvalidSha256Digest { field: &'static str },

    #[error(
        "deployment root verification receipt field `{field}` must be a supported timestamp label"
    )]
    InvalidTimestampLabel { field: &'static str },

    #[error("deployment root verification receipt field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },

    #[error("deployment root verification receipt state transition is inconsistent")]
    StateTransitionMismatch,

    #[error("deployment root verification receipt local state digests are inconsistent")]
    LocalStateDigestMismatch,

    #[error("deployment root verification receipt source evidence is inconsistent")]
    SourceEvidenceMismatch,
}
