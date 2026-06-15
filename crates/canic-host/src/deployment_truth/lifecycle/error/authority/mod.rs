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
