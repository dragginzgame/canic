use super::fleet_activation_journal::{
    ResolvedFleetInstallActivation, RootInstalledFleetInstallActivation,
    admit_root_install_receipt, record_root_installed,
};
use super::operations::InstallRootWasmOperation;
use super::options::InstallRootOptions;
use super::phase_receipts::InstallReceiptScope;
use super::plan_artifacts::{PreparedPlanArtifacts, normal_install_root_wasm};
use super::timing::InstallTimingSummary;
use crate::canister_build::WorkspaceBuildContext;

pub(super) struct PreparedRootInstall {
    pub(super) timings: InstallTimingSummary,
    pub(super) activation: RootInstalledFleetInstallActivation,
}

pub(super) fn install_root_prepared(
    receipt_scope: InstallReceiptScope<'_>,
    options: &InstallRootOptions,
    root_canister_id: &str,
    build_context: &WorkspaceBuildContext,
    plan_artifacts: Option<&PreparedPlanArtifacts>,
    activation: &ResolvedFleetInstallActivation,
) -> Result<PreparedRootInstall, Box<dyn std::error::Error>> {
    let mut timings = InstallTimingSummary::default();
    let root_wasm = match plan_artifacts {
        Some(artifacts) => artifacts.verified_root_wasm_path()?,
        None => normal_install_root_wasm(receipt_scope.icp_root, &options.root_build_target),
    };
    let install_operation = InstallRootWasmOperation::new(
        receipt_scope.icp_root,
        receipt_scope.environment,
        root_canister_id,
        root_wasm,
        &activation.journal.activation.identity,
        build_context.local_replica.as_ref(),
    )?;
    let completed_root_install =
        receipt_scope.run_operation_with_receipt(&install_operation, Some(root_canister_id))?;
    timings.install_root = completed_root_install.duration;
    let receipt = admit_root_install_receipt(&completed_root_install.receipt_path)?;
    let activation = record_root_installed(receipt_scope.icp_root, activation, &receipt)?;

    Ok(PreparedRootInstall {
        timings,
        activation,
    })
}
