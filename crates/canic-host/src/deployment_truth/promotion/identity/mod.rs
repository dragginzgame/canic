mod group;
mod report;
mod roles;
mod validation;

pub use report::{
    promotion_artifact_identity_report, promotion_artifact_identity_report_from_inputs,
};
pub use validation::validate_promotion_artifact_identity_report;

pub(super) use group::{
    materialization_output_key_for_group, materialization_output_key_for_role,
    promotion_materialization_output_groups,
};
pub(super) use roles::{
    artifact_identity_changed, role_materialization_identity_from_evidence,
    role_materialization_identity_matches, role_summary_artifact_identity_changed,
};
