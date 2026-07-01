mod digest;
mod error;
mod receipt;
mod report;

pub use digest::deployment_root_verification_receipt_digest;
pub use error::{DeploymentRootVerificationReceiptError, DeploymentRootVerificationReportError};
pub use receipt::validate_deployment_root_verification_receipt;
#[cfg(test)]
pub(in crate::deployment_truth) use report::{
    ROOT_VERIFICATION_CHECK_FAILED_CODE, ROOT_VERIFICATION_SOURCE_CHECK_DIFF_STALE_CODE,
    ROOT_VERIFICATION_SOURCE_CHECK_REPORT_STALE_CODE,
    ROOT_VERIFICATION_SOURCE_CHECK_SCHEMA_MISMATCH_CODE,
};
pub use report::{
    deployment_root_verification_report_from_check, validate_deployment_root_verification_report,
};
