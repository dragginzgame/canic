use crate::{deployment_truth::DeploymentReceiptV1, durable_io::write_bytes};
use canic_core::ids::FleetKey;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub(super) fn write_install_deployment_truth_receipt(
    icp_root: &Path,
    fleet: FleetKey,
    receipt: &DeploymentReceiptV1,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = install_deployment_truth_receipt_path(icp_root, fleet, receipt);
    let mut bytes = serde_json::to_vec_pretty(receipt)?;
    bytes.push(b'\n');
    write_bytes(&path, &bytes)?;
    Ok(path)
}

pub(super) fn install_deployment_truth_receipt_path(
    icp_root: &Path,
    fleet: FleetKey,
    receipt: &DeploymentReceiptV1,
) -> PathBuf {
    let file_stem = format!(
        "{}-{}",
        safe_deployment_truth_path_label(&receipt.started_at),
        safe_deployment_truth_path_label(&receipt.operation_id)
    );
    install_deployment_truth_receipts_dir(icp_root, fleet).join(format!("{file_stem}.json"))
}

/// Find the latest persisted deployment-truth receipt for one exact Fleet.
pub fn latest_deployment_truth_receipt_path_from_root(
    icp_root: &Path,
    fleet: FleetKey,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    let dir = install_deployment_truth_receipts_dir(icp_root, fleet);
    match fs::symlink_metadata(&dir) {
        Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => {}
        Ok(_) => {
            return Err(format!(
                "Fleet deployment-receipt path is not a regular directory: {}",
                dir.display()
            )
            .into());
        }
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(source.into()),
    }

    let mut latest = None;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_file()
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

pub(super) fn install_deployment_truth_receipts_dir(icp_root: &Path, fleet: FleetKey) -> PathBuf {
    icp_root
        .join(".canic")
        .join("networks")
        .join(fleet.canonical_network_id.to_string())
        .join("fleets")
        .join(fleet.fleet_id.to_string())
        .join("deployment-receipts")
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
