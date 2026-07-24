use super::state::{validate_environment_name, validate_state_name};
use crate::{deployment_truth::DeploymentReceiptV1, durable_io::write_bytes};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) fn write_install_deployment_truth_receipt(
    icp_root: &Path,
    environment: &str,
    deployment_name: &str,
    receipt: &DeploymentReceiptV1,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path =
        install_deployment_truth_receipt_path(icp_root, environment, deployment_name, receipt)?;
    let mut bytes = serde_json::to_vec_pretty(receipt)?;
    bytes.push(b'\n');
    write_bytes(&path, &bytes)?;
    Ok(path)
}

pub(super) fn install_deployment_truth_receipt_path(
    icp_root: &Path,
    environment: &str,
    deployment_name: &str,
    receipt: &DeploymentReceiptV1,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    validate_environment_name(environment)?;
    validate_state_name(deployment_name)?;
    let file_stem = format!(
        "{}-{}",
        safe_deployment_truth_path_label(&receipt.started_at),
        safe_deployment_truth_path_label(&receipt.operation_id)
    );
    Ok(
        install_deployment_truth_receipts_dir(icp_root, environment, deployment_name)?
            .join(format!("{file_stem}.json")),
    )
}

/// Find the latest persisted deployment-truth receipt for one local deployment target.
pub fn latest_deployment_truth_receipt_path_from_root(
    icp_root: &Path,
    environment: &str,
    deployment_name: &str,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    let dir = install_deployment_truth_receipts_dir(icp_root, environment, deployment_name)?;
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
    environment: &str,
    deployment_name: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    validate_environment_name(environment)?;
    validate_state_name(deployment_name)?;
    Ok(icp_root
        .join(".canic")
        .join(environment)
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
