mod check;
mod policy;
mod report;
mod shared;

pub use check::{
    external_upgrade_verification_check_from_policy,
    external_upgrade_verification_observation_from_check,
    validate_external_upgrade_verification_check,
    validate_external_upgrade_verification_check_for_deployment_check,
    validate_external_upgrade_verification_check_for_policy,
};
pub use policy::{
    external_upgrade_verification_policy_from_proposal,
    validate_external_upgrade_verification_policy,
    validate_external_upgrade_verification_policy_for_proposal,
};
pub use report::{
    external_upgrade_verification_report_from_receipt,
    validate_external_upgrade_verification_report,
    validate_external_upgrade_verification_report_for_receipt,
};
pub(in crate::deployment_truth::lifecycle::external_upgrade) use shared::{
    external_upgrade_verification_notes, external_upgrade_verification_result,
};
