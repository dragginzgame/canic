use super::state::{
    InstallState, RootVerificationStatus, deployment_install_state_path, write_install_state,
};
use crate::deployment_truth::{
    DeploymentRootVerificationReceiptV1, DeploymentRootVerificationReportV1,
    DeploymentRootVerificationStateTransitionV1, DeploymentRootVerificationStateV1,
    deployment_root_verification_receipt_digest, validate_deployment_root_verification_receipt,
};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

pub(super) struct RootVerificationReceiptInput {
    pub(super) deployment_name: String,
    pub(super) environment: String,
    pub(super) fleet_template: String,
    pub(super) root_principal: String,
    pub(super) previous_root_verification: DeploymentRootVerificationStateV1,
    pub(super) state_transition: DeploymentRootVerificationStateTransitionV1,
    pub(super) report: DeploymentRootVerificationReportV1,
    pub(super) verified_at_unix_secs: u64,
    pub(super) local_state_path: String,
    pub(super) local_state_digest_before: String,
    pub(super) local_state_digest_after: String,
}

pub(super) fn root_verification_receipt_from_report(
    input: RootVerificationReceiptInput,
) -> Result<DeploymentRootVerificationReceiptV1, Box<dyn std::error::Error>> {
    let source_root_observation_source = input.report.observed_root_observation_source.ok_or(
        "deployment root verification report did not preserve observed root source evidence",
    )?;
    let source_observed_root_canister_id =
        input.report.observed_root_canister_id.clone().ok_or(
            "deployment root verification report did not preserve observed root canister id",
        )?;

    let mut receipt = DeploymentRootVerificationReceiptV1 {
        schema_version: crate::deployment_truth::DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        receipt_id: format!(
            "local:{}:{}:root-verification-receipt",
            input.environment, input.deployment_name
        ),
        receipt_digest: String::new(),
        deployment_name: input.deployment_name,
        environment: input.environment,
        fleet_template: input.fleet_template,
        root_principal: input.root_principal,
        previous_root_verification: input.previous_root_verification,
        new_root_verification: DeploymentRootVerificationStateV1::Verified,
        state_transition: input.state_transition,
        source_report_id: input.report.report_id,
        source_report_digest: input.report.report_digest,
        source_report_requested_at: input.report.requested_at,
        source_report_source: input.report.source,
        source_report_evidence_status: input.report.evidence_status,
        source_report_current_root_verification: input.report.current_root_verification,
        source_report_state_transition: input.report.state_transition,
        source_root_observation_source,
        source_observed_root_canister_id,
        source_check_id: input.report.source_check_id,
        source_check_digest: input.report.source_check_digest,
        source_deployment_plan_id: input.report.source_deployment_plan_id,
        source_deployment_plan_digest: input.report.source_deployment_plan_digest,
        source_inventory_id: input.report.source_inventory_id,
        source_inventory_digest: input.report.source_inventory_digest,
        verified_at_unix_secs: input.verified_at_unix_secs,
        local_state_path: input.local_state_path,
        local_state_digest_before: input.local_state_digest_before,
        local_state_digest_after: input.local_state_digest_after,
        warnings: input.report.warnings,
    };
    receipt.receipt_digest = deployment_root_verification_receipt_digest(&receipt);
    validate_deployment_root_verification_receipt(&receipt)?;
    Ok(receipt)
}

pub(super) const fn deployment_root_verification_state(
    status: &RootVerificationStatus,
) -> DeploymentRootVerificationStateV1 {
    match status {
        RootVerificationStatus::Verified => DeploymentRootVerificationStateV1::Verified,
        RootVerificationStatus::NotVerified => DeploymentRootVerificationStateV1::NotVerified,
    }
}

pub(super) const fn verified_root_state_transition(
    previous: DeploymentRootVerificationStateV1,
) -> DeploymentRootVerificationStateTransitionV1 {
    match previous {
        DeploymentRootVerificationStateV1::NotVerified => {
            DeploymentRootVerificationStateTransitionV1::PromotedNotVerifiedToVerified
        }
        DeploymentRootVerificationStateV1::Verified => {
            DeploymentRootVerificationStateTransitionV1::NoStateChange
        }
    }
}

pub(super) fn write_verified_root_state_if_unchanged(
    icp_root: &Path,
    environment: &str,
    state: &InstallState,
    expected_digest_before: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let path = deployment_install_state_path(icp_root, environment, &state.deployment_name);
    let current_digest = file_sha256_hex(&path)?;
    if current_digest != expected_digest_before {
        return Err(format!(
            "deployment root verification state changed before write: expected {expected_digest_before}, found {current_digest}"
        )
        .into());
    }
    write_install_state(icp_root, environment, state)?;
    file_sha256_hex(&path)
}

pub(super) fn file_sha256_hex(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    Ok(bytes_sha256_hex(&fs::read(path)?))
}

fn bytes_sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}
