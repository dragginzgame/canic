use super::clock::current_unix_secs;
use super::root_verification::{
    RootVerificationReceiptInput, deployment_root_verification_state, file_sha256_hex,
    root_verification_receipt_from_report, verified_root_state_transition,
    write_verified_root_state_if_unchanged,
};
use super::state::{
    INSTALL_STATE_SCHEMA_VERSION, InstallState, RootVerificationStatus,
    deployment_install_state_path, read_deployment_install_state, validate_network_name,
    validate_state_name, write_install_state,
};
use crate::deployment_truth::{
    DeploymentCheckV1, DeploymentRootVerificationEvidenceStatusV1,
    DeploymentRootVerificationReceiptV1, DeploymentRootVerificationRequestV1,
    DeploymentRootVerificationSourceV1, DeploymentRootVerificationStateV1,
    deployment_root_verification_report_from_check, validate_deployment_root_verification_report,
};
use crate::release_set::{
    icp_root, resolve_artifact_root, root_release_set_manifest_path, workspace_root,
};
use canic_core::cdk::types::Principal;
use std::path::{Path, PathBuf};

///
/// RegisterDeploymentStateOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisterDeploymentStateOptions {
    pub deployment_name: String,
    pub fleet_template: String,
    pub root_canister_id: String,
    pub network: String,
    pub allow_unverified: bool,
    pub icp_root: Option<PathBuf>,
    pub workspace_root: Option<PathBuf>,
}

///
/// VerifyDeploymentRootOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifyDeploymentRootOptions {
    pub deployment_name: String,
    pub network: String,
    pub deployment_check: DeploymentCheckV1,
    pub verified_at_unix_secs: Option<u64>,
    pub icp_root: Option<PathBuf>,
}

/// Register minimal local deployment-target state for an existing root canister.
///
/// Registration is an explicit operator acknowledgement path. It does not
/// verify live inventory, copy receipts, or claim artifact/controller truth.
pub fn register_deployment_state(
    options: RegisterDeploymentStateOptions,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    validate_state_name(&options.deployment_name)?;
    validate_state_name(&options.fleet_template)?;
    validate_network_name(&options.network)?;
    if !options.allow_unverified {
        return Err(
            "deployment registration requires explicit unverified-root acknowledgement; pass --allow-unverified"
                .into(),
        );
    }
    Principal::from_text(&options.root_canister_id).map_err(|err| {
        format!(
            "invalid root principal for deployment {}: {err}",
            options.deployment_name
        )
    })?;

    let workspace_root = match options.workspace_root {
        Some(path) => path,
        None => workspace_root()?,
    };
    let icp_root = match options.icp_root {
        Some(path) => path,
        None => icp_root()?,
    };
    let release_set_manifest_path =
        registered_deployment_release_set_manifest_path(&icp_root, &options.network);
    let timestamp = current_unix_secs()?;
    let state = InstallState {
        schema_version: INSTALL_STATE_SCHEMA_VERSION,
        deployment_name: options.deployment_name,
        fleet_template: options.fleet_template.clone(),
        created_at_unix_secs: timestamp,
        updated_at_unix_secs: timestamp,
        network: options.network.clone(),
        root_target: options.root_canister_id.clone(),
        root_canister_id: options.root_canister_id,
        root_verification: RootVerificationStatus::NotVerified,
        root_build_target: "root".to_string(),
        workspace_root: workspace_root.display().to_string(),
        icp_root: icp_root.display().to_string(),
        config_path: workspace_root
            .join("fleets")
            .join(&options.fleet_template)
            .join("canic.toml")
            .display()
            .to_string(),
        release_set_manifest_path: release_set_manifest_path.display().to_string(),
    };

    Ok(write_install_state(&icp_root, &options.network, &state)?)
}

/// Promote an explicitly registered deployment root from `not_verified` to
/// `verified` using bound deployment-truth evidence.
pub fn verify_registered_deployment_root(
    options: VerifyDeploymentRootOptions,
) -> Result<DeploymentRootVerificationReceiptV1, Box<dyn std::error::Error>> {
    validate_state_name(&options.deployment_name)?;
    validate_network_name(&options.network)?;
    let verified_at_unix_secs = match options.verified_at_unix_secs {
        Some(value) => value,
        None => current_unix_secs()?,
    };
    let icp_root = match options.icp_root {
        Some(path) => path,
        None => icp_root()?,
    };
    let state_path =
        deployment_install_state_path(&icp_root, &options.network, &options.deployment_name);
    let state =
        read_deployment_install_state(&icp_root, &options.network, &options.deployment_name)?
            .ok_or_else(|| {
                format!(
                    "no local deployment state exists for {}; run canic deploy register first",
                    options.deployment_name
                )
            })?;
    let state_fleet_template = state.fleet_template.clone();
    let state_root_canister_id = state.root_canister_id.clone();
    let local_state_digest_before = file_sha256_hex(&state_path)?;
    let previous_root_verification = deployment_root_verification_state(&state.root_verification);
    let report =
        deployment_root_verification_report_from_check(DeploymentRootVerificationRequestV1 {
            report_id: format!(
                "local:{}:{}:root-verification-report",
                options.network, options.deployment_name
            ),
            requested_at: format!("unix:{verified_at_unix_secs}"),
            deployment_name: options.deployment_name.clone(),
            network: options.network.clone(),
            expected_fleet_template: state.fleet_template.clone(),
            expected_root_principal: state.root_canister_id.clone(),
            current_root_verification: previous_root_verification,
            source: DeploymentRootVerificationSourceV1::DeploymentTruthCheck,
            deployment_check: options.deployment_check,
        });
    validate_deployment_root_verification_report(&report)?;
    if report.evidence_status != DeploymentRootVerificationEvidenceStatusV1::EvidenceSatisfied {
        return Err(format!(
            "deployment root verification failed for {}: {} blocker(s)",
            options.deployment_name,
            report.blockers.len()
        )
        .into());
    }
    let state_transition = verified_root_state_transition(previous_root_verification);
    let local_state_digest_after = match previous_root_verification {
        DeploymentRootVerificationStateV1::NotVerified => {
            let mut verified_state = state;
            verified_state.root_verification = RootVerificationStatus::Verified;
            verified_state.updated_at_unix_secs = verified_at_unix_secs;
            write_verified_root_state_if_unchanged(
                &icp_root,
                &options.network,
                &verified_state,
                &local_state_digest_before,
            )?
        }
        DeploymentRootVerificationStateV1::Verified => file_sha256_hex(&state_path)?,
    };

    root_verification_receipt_from_report(RootVerificationReceiptInput {
        deployment_name: options.deployment_name,
        network: options.network,
        fleet_template: state_fleet_template,
        root_principal: state_root_canister_id,
        previous_root_verification,
        state_transition,
        report,
        verified_at_unix_secs,
        local_state_path: state_path.display().to_string(),
        local_state_digest_before,
        local_state_digest_after,
    })
}

fn registered_deployment_release_set_manifest_path(icp_root: &Path, network: &str) -> PathBuf {
    let artifact_root = resolve_artifact_root(icp_root, network)
        .unwrap_or_else(|_| icp_root.join(".icp").join(network).join("canisters"));
    root_release_set_manifest_path(&artifact_root)
        .unwrap_or_else(|_| artifact_root.join("root").join("root.release-set.json"))
}
