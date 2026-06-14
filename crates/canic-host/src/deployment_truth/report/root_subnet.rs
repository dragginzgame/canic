use super::super::*;
use super::{finding, refresh_resume_safety};
use serde::Deserialize;
use std::{path::Path, process::Command};

const MAINNET_NETWORK: &str = "ic";
const CLOUD_ENGINE_SUBNET_KIND: &str = "cloud_engine";
const DEFAULT_ICQ_EXECUTABLE: &str = "icq";
const CANIC_ICQ_ENV: &str = "CANIC_ICQ";

pub(in crate::deployment_truth) fn apply_root_canister_signature_subnet_check(
    diff: &mut DeploymentDiffV1,
    inventory: &DeploymentInventoryV1,
    network: &str,
    icp_root: &Path,
) {
    apply_root_canister_signature_subnet_check_with_source(
        diff,
        inventory,
        network,
        icp_root,
        &LiveIcqRootSubnetEvidenceSource,
    );
}

pub(in crate::deployment_truth) fn apply_root_canister_signature_subnet_check_with_source(
    diff: &mut DeploymentDiffV1,
    inventory: &DeploymentInventoryV1,
    network: &str,
    icp_root: &Path,
    source: &dyn RootSubnetEvidenceSource,
) {
    if network != MAINNET_NETWORK {
        return;
    }
    let Some(root) = &inventory.observed_root else {
        return;
    };
    let evidence = match source.root_subnet_evidence(network, icp_root, &root.observed_canister_id)
    {
        Ok(evidence) => evidence,
        Err(err) => {
            diff.hard_failures.push(finding(
                "root_auth_subnet_evidence_missing",
                format!(
                    "cannot verify root canister-signature subnet kind for {} with icq: {err}",
                    root.observed_canister_id
                ),
                SafetySeverityV1::HardFailure,
                Some(root.observed_canister_id.clone()),
            ));
            refresh_resume_safety(diff);
            return;
        }
    };
    if evidence.subnet_kind == CLOUD_ENGINE_SUBNET_KIND {
        diff.hard_failures.push(finding(
            "root_auth_cloud_engine_subnet",
            format!(
                "root canister {} resolves to cloud_engine subnet {}; IC canister signatures from cloud_engine subnets are invalid",
                root.observed_canister_id, evidence.subnet_principal
            ),
            SafetySeverityV1::HardFailure,
            Some(root.observed_canister_id.clone()),
        ));
        refresh_resume_safety(diff);
    }
}

///
/// RootSubnetEvidence
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::deployment_truth) struct RootSubnetEvidence {
    pub subnet_principal: String,
    pub subnet_kind: String,
}

pub(in crate::deployment_truth) trait RootSubnetEvidenceSource {
    fn root_subnet_evidence(
        &self,
        network: &str,
        icp_root: &Path,
        canister_id: &str,
    ) -> Result<RootSubnetEvidence, String>;
}

///
/// LiveIcqRootSubnetEvidenceSource
///
struct LiveIcqRootSubnetEvidenceSource;

impl RootSubnetEvidenceSource for LiveIcqRootSubnetEvidenceSource {
    fn root_subnet_evidence(
        &self,
        network: &str,
        icp_root: &Path,
        canister_id: &str,
    ) -> Result<RootSubnetEvidence, String> {
        let executable = icq_executable();
        let command_line =
            format!("{executable} --network {network} nns subnet info {canister_id} --format json");
        let output = Command::new(&executable)
            .current_dir(icp_root)
            .args([
                "--network",
                network,
                "nns",
                "subnet",
                "info",
                canister_id,
                "--format",
                "json",
            ])
            .output()
            .map_err(|err| format!("failed to run {command_line}: {err}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let detail = if stderr.is_empty() { stdout } else { stderr };
            return Err(format!("{command_line} failed: {detail}"));
        }
        let report =
            serde_json::from_slice::<IcqSubnetInfoReport>(&output.stdout).map_err(|err| {
                let stdout = String::from_utf8_lossy(&output.stdout);
                format!("failed to parse {command_line} JSON: {err}; output: {stdout}")
            })?;
        Ok(RootSubnetEvidence {
            subnet_principal: report.subnet_principal,
            subnet_kind: report.subnet_kind,
        })
    }
}

#[derive(Deserialize)]
struct IcqSubnetInfoReport {
    subnet_principal: String,
    subnet_kind: String,
}

fn icq_executable() -> String {
    std::env::var(CANIC_ICQ_ENV)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_ICQ_EXECUTABLE.to_string())
}
