use super::fleet_activation_journal::{
    CanistersPreparedFleetInstallActivation, FleetInstallActivationJournalError,
    FleetInstallActivationPhase, ResolvedFleetInstallActivation, admit_canisters_prepared,
    admit_root_install_receipt, record_canisters_prepared, record_root_installed,
    recover_root_install_receipt, resume_canisters_prepared,
};
use super::operations::InstallRootWasmOperation;
use super::options::InstallRootOptions;
use super::phase_receipts::InstallReceiptScope;
use super::plan_artifacts::{PreparedPlanArtifacts, normal_install_root_wasm};
use super::receipt_io::install_deployment_truth_receipts_dir;
use super::timing::InstallTimingSummary;
use crate::{
    canister_build::WorkspaceBuildContext,
    icp::{IcpCli, decode_json_result_response},
};
use canic_core::{dto::fleet_activation::FleetActivationStatusResponse, protocol};

pub(super) struct PreparedRootInstall {
    pub(super) timings: InstallTimingSummary,
    pub(super) activation: CanistersPreparedFleetInstallActivation,
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
    if activation.journal.phase == FleetInstallActivationPhase::CanistersPrepared {
        return Ok(PreparedRootInstall {
            timings,
            activation: resume_canisters_prepared(activation)?,
        });
    }
    let resolved_root = root_canister_id.parse()?;
    let receipt = match activation.journal.phase {
        FleetInstallActivationPhase::Planned => {
            let root_wasm = match plan_artifacts {
                Some(artifacts) => artifacts.verified_root_wasm_path()?,
                None => {
                    normal_install_root_wasm(receipt_scope.icp_root, &options.root_build_target)
                }
            };
            let install_operation = InstallRootWasmOperation::new(
                receipt_scope.icp_root,
                receipt_scope.environment,
                root_canister_id,
                root_wasm,
                &activation.journal.activation.identity,
                build_context.local_replica.as_ref(),
            )?;
            let completed_root_install = receipt_scope
                .run_operation_with_receipt(&install_operation, Some(root_canister_id))?;
            timings.install_root = completed_root_install.duration;
            admit_root_install_receipt(&completed_root_install.receipt_path)?
        }
        FleetInstallActivationPhase::RootInstalled => recover_root_install_receipt(
            &install_deployment_truth_receipts_dir(receipt_scope.icp_root, receipt_scope.fleet),
            activation
                .journal
                .root_install_receipt_hash
                .expect("validated RootInstalled journal retains its receipt hash"),
        )?,
        phase => {
            return Err(Box::new(
                FleetInstallActivationJournalError::InvalidCanistersPreparedTransition { phase },
            ));
        }
    };
    if receipt.root_canister != resolved_root {
        return Err(Box::new(
            FleetInstallActivationJournalError::RootInstallReceiptCanisterMismatch {
                receipt_root: receipt.root_canister,
                resolved_root,
            },
        ));
    }
    let root_installed = record_root_installed(receipt_scope.icp_root, activation, &receipt)?;
    let icp = IcpCli::new("icp", Some(receipt_scope.environment.to_string()))
        .with_cwd(receipt_scope.icp_root)
        .with_local_replica(build_context.local_replica.clone());
    let output = icp.canister_call_arg_output_with_candid(
        root_canister_id,
        protocol::CANIC_PREPARE_FLEET_ACTIVATION,
        "()",
        Some("json"),
        None,
    )?;
    let root_status = decode_json_result_response::<FleetActivationStatusResponse>(&output)?;
    // The root returns only after every synchronous state/topology cascade has
    // acknowledged durable Prepared evidence. The host journals that exact
    // root-owned manifest; it does not assume the operator controls children.
    let evidence = admit_canisters_prepared(
        receipt.root_canister,
        &root_installed.journal.activation.identity,
        &root_status,
    )?;
    let activation = record_canisters_prepared(receipt_scope.icp_root, &root_installed, &evidence)?;

    Ok(PreparedRootInstall {
        timings,
        activation,
    })
}
