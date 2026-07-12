use super::operations::{EmitRootManifestOperation, InstallPhaseLabel};
use super::phase_receipts::{
    CompletedInstallPhase, install_deployment_truth_phase_receipt, receipt_with_execution_context,
};
use super::receipt_io::write_install_deployment_truth_receipt;
use super::{clock::current_unix_timestamp_label, options::InstallRootOptions};
use crate::deployment_truth::{DeploymentCheckV1, DeploymentExecutionContextV1, DeploymentPlanV1};
use crate::release_set::{
    ReleaseSetEntry, RootReleaseSetManifest, resolve_artifact_root, resolve_release_artifact_path,
    root_release_set_manifest_path, validate_root_release_set_manifest,
};
use canic_core::CANIC_WASM_CHUNK_BYTES;
use canic_core::cdk::utils::hash::wasm_hash_hex;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub(super) fn validate_plan_artifacts_with_phase(
    plan: &DeploymentPlanV1,
    icp_root: &Path,
    network: &str,
) -> Result<(CompletedInstallPhase, Duration), Box<dyn std::error::Error>> {
    let started_at = current_unix_timestamp_label()?;
    let started = Instant::now();
    validate_plan_artifact_paths(plan, icp_root, network)?;
    let duration = started.elapsed();
    let role_names = plan
        .role_artifacts
        .iter()
        .map(|artifact| artifact.role.clone())
        .collect::<Vec<_>>();
    let phase = CompletedInstallPhase {
        phase: InstallPhaseLabel::MATERIALIZE_ARTIFACTS,
        attempted_action: "validate supplied deployment plan artifacts",
        started_at,
        finished_at: Some(current_unix_timestamp_label()?),
        evidence: vec![format!("deployment_plan:{}", plan.plan_id)],
        role_names,
    };
    Ok((phase, duration))
}

pub(super) fn emit_manifest_with_deployment_truth_receipt(
    workspace_root: &Path,
    icp_root: &Path,
    options: &InstallRootOptions,
    config_path: &Path,
    deployment_name: &str,
    deployment_truth_check: &DeploymentCheckV1,
    execution_context: &DeploymentExecutionContextV1,
) -> Result<(PathBuf, Duration), Box<dyn std::error::Error>> {
    let operation =
        EmitRootManifestOperation::new(workspace_root, icp_root, &options.network, config_path);
    let emit_manifest_started_at_label = current_unix_timestamp_label()?;
    let emit_manifest_started_at = Instant::now();
    let manifest_path = if let Some(plan) = &options.deployment_plan_override {
        emit_root_release_set_manifest_from_plan(icp_root, &options.network, plan)?
    } else {
        operation.execute()?
    };
    let emit_manifest_duration = emit_manifest_started_at.elapsed();
    let emit_manifest_receipt = receipt_with_execution_context(
        install_deployment_truth_phase_receipt(
            deployment_truth_check,
            InstallPhaseLabel::EMIT_MANIFEST,
            emit_manifest_started_at_label,
            Some(current_unix_timestamp_label()?),
            "emit root release-set manifest",
            crate::deployment_truth::ObservationStatusV1::Observed,
            EmitRootManifestOperation::evidence(&manifest_path),
        ),
        execution_context,
    );
    let emit_manifest_receipt_path = write_install_deployment_truth_receipt(
        icp_root,
        &options.network,
        deployment_name,
        &emit_manifest_receipt,
    )?;
    println!(
        "Deployment truth receipt JSON: {}",
        emit_manifest_receipt_path.display()
    );
    Ok((manifest_path, emit_manifest_duration))
}

pub(super) fn root_wasm_for_install_plan(
    icp_root: &Path,
    network: &str,
    root_build_target: &str,
    plan: Option<&DeploymentPlanV1>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let artifact_root = resolve_artifact_root(icp_root, network)?;
    if let Some(plan) = plan {
        let root_artifact = plan
            .role_artifacts
            .iter()
            .find(|artifact| artifact.role == "root")
            .ok_or_else(|| "deployment plan is missing root role artifact".to_string())?;
        return Ok(plan_role_wasm_path(icp_root, &artifact_root, root_artifact));
    }

    Ok(artifact_root
        .join(root_build_target)
        .join(format!("{root_build_target}.wasm")))
}

fn validate_plan_artifact_paths(
    plan: &DeploymentPlanV1,
    icp_root: &Path,
    network: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let artifact_root = resolve_artifact_root(icp_root, network)?;
    let root_artifact = plan
        .role_artifacts
        .iter()
        .find(|artifact| artifact.role == "root")
        .ok_or_else(|| "deployment plan is missing root role artifact".to_string())?;
    let root_wasm = plan_role_wasm_path(icp_root, &artifact_root, root_artifact);
    if !root_wasm.is_file() {
        return Err(format!(
            "deployment plan root wasm artifact does not exist: {}",
            root_wasm.display()
        )
        .into());
    }

    for artifact in plan_release_role_artifacts(plan) {
        let wasm_gz = plan_role_wasm_gz_path(icp_root, &artifact_root, artifact);
        if !wasm_gz.is_file() {
            return Err(format!(
                "deployment plan role {} wasm.gz artifact does not exist: {}",
                artifact.role,
                wasm_gz.display()
            )
            .into());
        }
    }
    Ok(())
}

fn emit_root_release_set_manifest_from_plan(
    icp_root: &Path,
    network: &str,
    plan: &DeploymentPlanV1,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let artifact_root = resolve_artifact_root(icp_root, network)?;
    let manifest_path = root_release_set_manifest_path(&artifact_root)?;
    let entries = plan_release_role_artifacts(plan)
        .map(|artifact| release_set_entry_from_plan_artifact(icp_root, &artifact_root, artifact))
        .collect::<Result<Vec<_>, _>>()?;
    let manifest = RootReleaseSetManifest {
        release_version: plan
            .deployment_identity
            .canic_version
            .clone()
            .unwrap_or_else(|| plan.plan_id.clone()),
        entries,
    };

    validate_root_release_set_manifest(&manifest)?;
    crate::durable_io::write_bytes(&manifest_path, &serde_json::to_vec_pretty(&manifest)?)?;
    Ok(manifest_path)
}

fn release_set_entry_from_plan_artifact(
    icp_root: &Path,
    artifact_root: &Path,
    artifact: &crate::deployment_truth::RoleArtifactV1,
) -> Result<ReleaseSetEntry, Box<dyn std::error::Error>> {
    let artifact_path = plan_role_wasm_gz_path(icp_root, artifact_root, artifact);
    let artifact_relative_path = artifact_path
        .strip_prefix(icp_root)
        .map_err(|_| {
            format!(
                "deployment plan artifact {} is not under ICP root {}",
                artifact_path.display(),
                icp_root.display()
            )
        })?
        .to_string_lossy()
        .to_string();
    let artifact_path = resolve_release_artifact_path(icp_root, &artifact_relative_path)?;
    let artifact_bytes = fs::read(&artifact_path)?;
    let chunk_hashes = artifact_bytes
        .chunks(CANIC_WASM_CHUNK_BYTES)
        .map(wasm_hash_hex)
        .collect::<Vec<_>>();

    Ok(ReleaseSetEntry {
        role: artifact.role.clone(),
        template_id: format!("embedded:{}", artifact.role),
        artifact_relative_path,
        payload_size_bytes: u64::try_from(artifact_bytes.len())?,
        payload_sha256_hex: wasm_hash_hex(&artifact_bytes),
        chunk_size_bytes: u64::try_from(CANIC_WASM_CHUNK_BYTES)?,
        chunk_sha256_hex: chunk_hashes,
    })
}

fn plan_release_role_artifacts(
    plan: &DeploymentPlanV1,
) -> impl Iterator<Item = &crate::deployment_truth::RoleArtifactV1> {
    plan.role_artifacts
        .iter()
        .filter(|artifact| !matches!(artifact.role.as_str(), "root" | "wasm_store"))
}

fn plan_role_wasm_path(
    icp_root: &Path,
    artifact_root: &Path,
    artifact: &crate::deployment_truth::RoleArtifactV1,
) -> PathBuf {
    artifact.wasm_path.as_ref().map_or_else(
        || {
            artifact_root
                .join(&artifact.role)
                .join(format!("{}.wasm", artifact.role))
        },
        |path| plan_artifact_path(icp_root, path),
    )
}

fn plan_role_wasm_gz_path(
    icp_root: &Path,
    artifact_root: &Path,
    artifact: &crate::deployment_truth::RoleArtifactV1,
) -> PathBuf {
    artifact.wasm_gz_path.as_ref().map_or_else(
        || {
            artifact_root
                .join(&artifact.role)
                .join(format!("{}.wasm.gz", artifact.role))
        },
        |path| plan_artifact_path(icp_root, path),
    )
}

fn plan_artifact_path(icp_root: &Path, path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        icp_root.join(path)
    }
}
