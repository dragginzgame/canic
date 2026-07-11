use super::state::{validate_network_name, validate_state_name};
use crate::{
    deployment_truth::{ArtifactPromotionExecutionReceiptV1, DeploymentReceiptV1},
    durable_io::write_bytes,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) fn write_install_deployment_truth_receipt(
    icp_root: &Path,
    network: &str,
    deployment_name: &str,
    receipt: &DeploymentReceiptV1,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = install_deployment_truth_receipt_path(icp_root, network, deployment_name, receipt)?;
    let mut bytes = serde_json::to_vec_pretty(receipt)?;
    bytes.push(b'\n');
    write_bytes(&path, &bytes)?;
    Ok(path)
}

pub(super) fn write_artifact_promotion_execution_receipt(
    icp_root: &Path,
    network: &str,
    deployment_name: &str,
    receipt: &ArtifactPromotionExecutionReceiptV1,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path =
        artifact_promotion_execution_receipt_path(icp_root, network, deployment_name, receipt)?;
    let mut bytes = serde_json::to_vec_pretty(receipt)?;
    bytes.push(b'\n');
    write_bytes(&path, &bytes)?;
    Ok(path)
}

fn artifact_promotion_execution_receipt_path(
    icp_root: &Path,
    network: &str,
    deployment_name: &str,
    receipt: &ArtifactPromotionExecutionReceiptV1,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    validate_network_name(network)?;
    validate_state_name(deployment_name)?;
    let file_stem = format!(
        "{}-{}",
        safe_deployment_truth_path_label(&receipt.started_at),
        safe_deployment_truth_path_label(&receipt.receipt_id)
    );
    Ok(
        artifact_promotion_execution_receipts_dir(icp_root, network, deployment_name)?
            .join(format!("{file_stem}.json")),
    )
}

fn artifact_promotion_execution_receipts_dir(
    icp_root: &Path,
    network: &str,
    deployment_name: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    validate_network_name(network)?;
    validate_state_name(deployment_name)?;
    Ok(icp_root
        .join(".canic")
        .join(network)
        .join("artifact-promotion-execution-receipts")
        .join(deployment_name))
}

pub(super) fn install_deployment_truth_receipt_path(
    icp_root: &Path,
    network: &str,
    deployment_name: &str,
    receipt: &DeploymentReceiptV1,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    validate_network_name(network)?;
    validate_state_name(deployment_name)?;
    let file_stem = format!(
        "{}-{}",
        safe_deployment_truth_path_label(&receipt.started_at),
        safe_deployment_truth_path_label(&receipt.operation_id)
    );
    Ok(
        install_deployment_truth_receipts_dir(icp_root, network, deployment_name)?
            .join(format!("{file_stem}.json")),
    )
}

/// Find the latest persisted deployment-truth receipt for one local deployment target.
pub fn latest_deployment_truth_receipt_path_from_root(
    icp_root: &Path,
    network: &str,
    deployment_name: &str,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    let dir = install_deployment_truth_receipts_dir(icp_root, network, deployment_name)?;
    if !dir.is_dir() {
        return Ok(None);
    }

    let mut latest = None;
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if !path.is_file()
            || path
                .extension()
                .is_none_or(|ext| !ext.eq_ignore_ascii_case("json"))
        {
            continue;
        }
        if latest.as_ref().is_none_or(|current| path > *current) {
            latest = Some(path);
        }
    }
    Ok(latest)
}

fn install_deployment_truth_receipts_dir(
    icp_root: &Path,
    network: &str,
    deployment_name: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    validate_network_name(network)?;
    validate_state_name(deployment_name)?;
    Ok(icp_root
        .join(".canic")
        .join(network)
        .join("deployment-receipts")
        .join(deployment_name))
}

fn safe_deployment_truth_path_label(value: &str) -> String {
    let label = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if label.is_empty() {
        "unknown".to_string()
    } else {
        label
    }
}
