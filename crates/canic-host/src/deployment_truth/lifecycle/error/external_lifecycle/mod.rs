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
