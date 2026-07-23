//! Module: install_root::plan_artifacts
//!
//! Responsibility: admit supplied-plan artifact bytes into one canonical install snapshot.
//! Does not own: deployment truth policy, network mutation, or activation sequencing.
//! Boundary: truth, manifest, and activation consumers receive the same prepared authority.

mod error;
mod prepared;

use crate::{
    canister_build::CurrentCanisterArtifactBuildOutput,
    deployment_truth::DeploymentPlanV1,
    install_root::{
        build_snapshot::ValidatedInstallSnapshot,
        clock::current_unix_timestamp_label,
        operations::{EmitRootManifestOperation, InstallPhaseLabel},
        options::InstallRootOptions,
        phase_receipts::{
            CompletedInstallPhase, InstallReceiptScope, install_deployment_truth_phase_receipt,
            receipt_with_execution_context,
        },
        receipt_io::write_install_deployment_truth_receipt,
    },
    release_build::{FinalizedReleaseBuild, finalize_release_build_from_manifest},
    release_set::artifact_root_path,
};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub(super) use prepared::PreparedPlanArtifacts;

#[cfg(test)]
pub(super) use error::PlanArtifactError;

pub(super) fn prepare_plan_artifacts_with_phase(
    plan: &DeploymentPlanV1,
    icp_root: &Path,
    environment: &str,
) -> Result<(PreparedPlanArtifacts, CompletedInstallPhase, Duration), Box<dyn std::error::Error>> {
    let started_at = current_unix_timestamp_label()?;
    let started = Instant::now();
    let prepared = PreparedPlanArtifacts::materialize(plan, icp_root, environment)?;
    let duration = started.elapsed();
    let role_names = prepared
        .plan()
        .role_artifacts
        .iter()
        .map(|artifact| artifact.role.clone())
        .collect::<Vec<_>>();
    let phase = CompletedInstallPhase {
        phase: InstallPhaseLabel::MATERIALIZE_ARTIFACTS,
        attempted_action: "verify and materialize supplied deployment plan artifacts",
        started_at,
        finished_at: Some(current_unix_timestamp_label()?),
        evidence: vec![format!("deployment_plan:{}", prepared.plan().plan_id)],
        role_names,
    };
    Ok((prepared, phase, duration))
}

pub(super) fn emit_manifest_with_deployment_truth_receipt(
    receipt_scope: InstallReceiptScope<'_>,
    options: &InstallRootOptions,
    install_snapshot: &ValidatedInstallSnapshot,
    build_outputs: &[CurrentCanisterArtifactBuildOutput],
    plan_artifacts: Option<&PreparedPlanArtifacts>,
) -> Result<(PathBuf, Duration, Option<FinalizedReleaseBuild>), Box<dyn std::error::Error>> {
    let emit_manifest_started_at_label = current_unix_timestamp_label()?;
    let emit_manifest_started_at = Instant::now();
    let manifest_path = if let Some(plan_artifacts) = plan_artifacts {
        plan_artifacts.emit_release_set_manifest()?
    } else {
        let complete_build = install_snapshot
            .complete_build
            .as_ref()
            .ok_or_else(|| "normal install is missing its complete-build snapshot".to_string())?;
        let operation = EmitRootManifestOperation::new(&complete_build.manifest, build_outputs);
        operation.execute()?
    };
    let emit_manifest_duration = emit_manifest_started_at.elapsed();
    let finalized_release_build = install_snapshot
        .release_build
        .as_ref()
        .map(|planned| {
            finalize_release_build_from_manifest(
                receipt_scope.icp_root,
                planned.record.release_build_id,
                &manifest_path,
            )
        })
        .transpose()?;
    let execution_context = receipt_scope
        .execution_context
        .ok_or_else(|| "manifest receipt requires an execution context".to_string())?;
    let emit_manifest_receipt = receipt_with_execution_context(
        install_deployment_truth_phase_receipt(
            receipt_scope.check,
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
        receipt_scope.icp_root,
        &options.environment,
        receipt_scope.deployment_name,
        &emit_manifest_receipt,
    )?;
    println!(
        "Deployment truth receipt JSON: {}",
        emit_manifest_receipt_path.display()
    );
    Ok((
        manifest_path,
        emit_manifest_duration,
        finalized_release_build,
    ))
}

pub(super) fn normal_install_root_wasm(icp_root: &Path, root_build_target: &str) -> PathBuf {
    artifact_root_path(icp_root, "local")
        .join(root_build_target)
        .join(format!("{root_build_target}.wasm"))
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;
    use std::fs;

    #[test]
    fn normal_install_uses_current_local_root_wasm() {
        let icp_root = temp_dir("canic-install-root-artifact-authority");

        assert_eq!(
            normal_install_root_wasm(&icp_root, "root"),
            icp_root.join(".icp/local/canisters/root/root.wasm")
        );
        let _ = fs::remove_dir_all(icp_root);
    }
}
