mod build;
mod checks;
mod shared;
mod validation;

pub use build::deployment_root_verification_report_from_check;
#[cfg(test)]
pub(in crate::deployment_truth) use checks::{
    ROOT_VERIFICATION_CHECK_FAILED_CODE, ROOT_VERIFICATION_SOURCE_CHECK_DIFF_STALE_CODE,
    ROOT_VERIFICATION_SOURCE_CHECK_REPORT_STALE_CODE,
    ROOT_VERIFICATION_SOURCE_CHECK_SCHEMA_MISMATCH_CODE,
};
pub use validation::validate_deployment_root_verification_report;
