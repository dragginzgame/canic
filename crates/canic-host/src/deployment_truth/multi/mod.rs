mod diff;
mod digest;
mod report;
mod status;
mod validation;

///
/// DeploymentComparisonReportError
///
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum DeploymentComparisonReportError {
    #[error(
        "deployment comparison report schema version {actual} does not match expected {expected}"
    )]
    SchemaVersionMismatch { expected: u32, actual: u32 },
    #[error("deployment comparison report field `{field}` is required")]
    MissingRequiredField { field: &'static str },
    #[error("deployment comparison report field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },
    #[error("deployment comparison report status does not match report findings")]
    StatusMismatch,
}

pub use report::deployment_comparison_report_from_checks;
#[cfg(test)]
pub(in crate::deployment_truth) use status::{
    DEPLOYMENT_COMPARISON_DRIFT_CODE, DEPLOYMENT_COMPARISON_INPUT_BLOCKED_CODE,
    DEPLOYMENT_COMPARISON_INPUT_DIFF_STALE_CODE, DEPLOYMENT_COMPARISON_INPUT_REPORT_STALE_CODE,
    DEPLOYMENT_COMPARISON_INPUT_WARNING_CODE,
};
pub use validation::validate_deployment_comparison_report;
