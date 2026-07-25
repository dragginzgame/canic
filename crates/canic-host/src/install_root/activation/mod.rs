use super::clock::current_unix_secs;
use super::fleet_activation_journal::{
    ActivatedFleetActivationEvidence, CanistersActivatedFleetInstallActivation,
    CanistersPreparedFleetInstallActivation, FleetInstallActivationJournalError,
    FleetInstallActivationPhase, ResolvedFleetInstallActivation, admit_canisters_activated,
    admit_canisters_prepared, admit_root_install_receipt, canisters_prepared_resume_request,
    observe_host_authority_committed, record_canisters_activated, record_canisters_prepared,
    record_host_authority_committed, record_root_installed, recover_root_install_receipt,
    resume_canisters_activated, resume_canisters_prepared,
};
use super::operations::InstallRootWasmOperation;
use super::options::InstallRootOptions;
use super::phase_receipts::InstallReceiptScope;
use super::plan_artifacts::{PreparedPlanArtifacts, normal_install_root_wasm};
use super::receipt_io::install_deployment_truth_receipts_dir;
use super::timing::InstallTimingSummary;
use crate::{
    canister_build::WorkspaceBuildContext,
    fleet_catalog::{
        FleetCatalogEntryV1, FleetCatalogError, commit_fleet_catalog_entry,
        read_fleet_catalog_entry_for_network,
    },
    icp::{IcpCli, decode_json_result_response},
};
use candid::IDLValue;
use canic_core::{
    dto::fleet_activation::{FleetActivationResumeRequest, FleetActivationStatusResponse},
    protocol,
};
use std::path::Path;
use thiserror::Error as ThisError;

pub(super) struct ActivatedRootInstall {
    pub(super) timings: InstallTimingSummary,
    pub(super) activation: CanistersActivatedFleetInstallActivation,
}

pub(super) struct CommittedRootInstall {
    pub(super) timings: InstallTimingSummary,
}

#[derive(Debug, ThisError)]
#[error(
    "root Fleet activation {operation_name} failed and its exact status could not be reconciled: operation={operation}; reconciliation={reconciliation}"
)]
struct FleetActivationCallReconciliationError {
    operation_name: &'static str,
    #[source]
    operation: Box<dyn std::error::Error>,
    reconciliation: Box<dyn std::error::Error>,
}

pub(super) fn install_root_committed(
    receipt_scope: InstallReceiptScope<'_>,
    options: &InstallRootOptions,
    root_canister_id: &str,
    build_context: &WorkspaceBuildContext,
    plan_artifacts: Option<&PreparedPlanArtifacts>,
    activation: &ResolvedFleetInstallActivation,
) -> Result<CommittedRootInstall, Box<dyn std::error::Error>> {
    let receipt_directory =
        install_deployment_truth_receipts_dir(receipt_scope.icp_root, receipt_scope.fleet);
    if activation.journal.phase == FleetInstallActivationPhase::HostAuthorityCommitted {
        let catalog_entry = read_fleet_catalog_entry_for_network(
            receipt_scope.icp_root,
            receipt_scope.fleet.network,
            &activation.journal.fleet_name,
        )?
        .ok_or_else(|| FleetCatalogError::UnknownFleet {
            canonical_network_id: receipt_scope.fleet.network,
            fleet_name: activation.journal.fleet_name.clone(),
        })?;
        observe_host_authority_committed(activation, &receipt_directory, &catalog_entry)?;
        return Ok(CommittedRootInstall {
            timings: InstallTimingSummary::default(),
        });
    }

    let activated = install_root_activated(
        receipt_scope,
        options,
        root_canister_id,
        build_context,
        plan_artifacts,
        activation,
    )?;
    let catalog = commit_fleet_catalog_entry(
        receipt_scope.icp_root,
        FleetCatalogEntryV1 {
            canonical_network_id: receipt_scope.fleet.network,
            fleet_id: receipt_scope.fleet.fleet_id,
            fleet_name: activated.activation.journal.fleet_name.clone(),
            app: activated
                .activation
                .journal
                .activation
                .identity
                .fleet
                .app
                .clone(),
            environment: receipt_scope.environment.to_string(),
            deployed_at_unix_secs: current_unix_secs()?,
            root_principal: root_canister_id.to_string(),
        },
    )?;
    record_host_authority_committed(
        receipt_scope.icp_root,
        &activated.activation,
        &receipt_directory,
        &catalog,
    )?;
    Ok(CommittedRootInstall {
        timings: activated.timings,
    })
}

pub(super) fn install_root_activated(
    receipt_scope: InstallReceiptScope<'_>,
    options: &InstallRootOptions,
    root_canister_id: &str,
    build_context: &WorkspaceBuildContext,
    plan_artifacts: Option<&PreparedPlanArtifacts>,
    activation: &ResolvedFleetInstallActivation,
) -> Result<ActivatedRootInstall, Box<dyn std::error::Error>> {
    if activation.journal.phase == FleetInstallActivationPhase::CanistersActivated {
        return Ok(ActivatedRootInstall {
            timings: InstallTimingSummary::default(),
            activation: resume_canisters_activated(activation)?,
        });
    }
    let prepared = install_root_prepared(
        receipt_scope,
        options,
        root_canister_id,
        build_context,
        plan_artifacts,
        activation,
    )?;
    let root_canister = root_canister_id.parse()?;
    let evidence = resume_and_admit_activation(
        receipt_scope.icp_root,
        receipt_scope.environment,
        root_canister_id,
        build_context,
        root_canister,
        &prepared.activation,
    )?;
    let activation =
        record_canisters_activated(receipt_scope.icp_root, &prepared.activation, &evidence)?;

    Ok(ActivatedRootInstall {
        timings: prepared.timings,
        activation,
    })
}

fn install_root_prepared(
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
    // The root returns only after every synchronous state/topology cascade has
    // acknowledged durable Prepared evidence. The host journals that exact
    // root-owned manifest; it does not assume the operator controls children.
    let evidence = match call_prepare(&icp, root_canister_id) {
        Ok(status) => admit_canisters_prepared(
            receipt.root_canister,
            &root_installed.journal.activation.identity,
            &status,
        )?,
        Err(operation) => {
            let reconciled = query_status(&icp, root_canister_id).and_then(|status| {
                admit_canisters_prepared(
                    receipt.root_canister,
                    &root_installed.journal.activation.identity,
                    &status,
                )
                .map_err(Into::into)
            });
            match reconciled {
                Ok(evidence) => evidence,
                Err(reconciliation) => {
                    return Err(Box::new(FleetActivationCallReconciliationError {
                        operation_name: "preparation",
                        operation,
                        reconciliation,
                    }));
                }
            }
        }
    };
    let activation = record_canisters_prepared(receipt_scope.icp_root, &root_installed, &evidence)?;

    Ok(PreparedRootInstall {
        timings,
        activation,
    })
}

struct PreparedRootInstall {
    timings: InstallTimingSummary,
    activation: CanistersPreparedFleetInstallActivation,
}

fn resume_and_admit_activation(
    icp_root: &Path,
    environment: &str,
    root_canister_id: &str,
    build_context: &WorkspaceBuildContext,
    root_canister: canic_core::cdk::types::Principal,
    prepared: &CanistersPreparedFleetInstallActivation,
) -> Result<ActivatedFleetActivationEvidence, Box<dyn std::error::Error>> {
    let icp = IcpCli::new("icp", Some(environment.to_string()))
        .with_cwd(icp_root)
        .with_local_replica(build_context.local_replica.clone());
    let request = canisters_prepared_resume_request(prepared);
    match call_resume(&icp, root_canister_id, &request) {
        Ok(status) => {
            admit_canisters_activated(root_canister, prepared, &status).map_err(Into::into)
        }
        Err(operation) => {
            let reconciled = query_status(&icp, root_canister_id).and_then(|status| {
                admit_canisters_activated(root_canister, prepared, &status).map_err(Into::into)
            });
            match reconciled {
                Ok(evidence) => Ok(evidence),
                Err(reconciliation) => Err(Box::new(FleetActivationCallReconciliationError {
                    operation_name: "resume",
                    operation,
                    reconciliation,
                })),
            }
        }
    }
}

fn call_prepare(
    icp: &IcpCli,
    root_canister_id: &str,
) -> Result<FleetActivationStatusResponse, Box<dyn std::error::Error>> {
    let output = icp.canister_call_arg_output_with_candid(
        root_canister_id,
        protocol::CANIC_PREPARE_FLEET_ACTIVATION,
        "()",
        Some("json"),
        None,
    )?;
    decode_json_result_response(&output).map_err(Into::into)
}

fn call_resume(
    icp: &IcpCli,
    root_canister_id: &str,
    request: &FleetActivationResumeRequest,
) -> Result<FleetActivationStatusResponse, Box<dyn std::error::Error>> {
    let output = icp.canister_call_arg_output_with_candid(
        root_canister_id,
        protocol::CANIC_RESUME_FLEET_ACTIVATION,
        &fleet_activation_resume_args(request)?,
        Some("json"),
        None,
    )?;
    decode_json_result_response(&output).map_err(Into::into)
}

fn fleet_activation_resume_args(
    request: &FleetActivationResumeRequest,
) -> Result<String, candid::Error> {
    let value = IDLValue::try_from_candid_type(request)?;
    Ok(format!("({value})"))
}

fn query_status(
    icp: &IcpCli,
    root_canister_id: &str,
) -> Result<FleetActivationStatusResponse, Box<dyn std::error::Error>> {
    let output = icp.canister_query_output_with_candid(
        root_canister_id,
        protocol::CANIC_FLEET_ACTIVATION_STATUS,
        Some("json"),
        None,
    )?;
    decode_json_result_response(&output).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::{CandidType, TypeEnv};
    use canic_core::dto::fleet_activation::FleetCredentialGenerationRef;

    #[test]
    fn resume_args_roundtrip_the_exact_journal_request() {
        let request = FleetActivationResumeRequest {
            operation_id: [7; 32],
            credential: FleetCredentialGenerationRef {
                generation: 8,
                manifest_hash: [9; 32],
            },
        };

        let args = fleet_activation_resume_args(&request).expect("build resume args");
        let parsed = candid_parser::parse_idl_args(&args).expect("parse textual Candid");
        let bytes = parsed
            .to_bytes_with_types(&TypeEnv::new(), &[FleetActivationResumeRequest::ty()])
            .expect("encode typed textual Candid");
        let decoded: FleetActivationResumeRequest =
            candid::decode_one(&bytes).expect("decode resume request");

        assert_eq!(decoded, request);
    }
}
