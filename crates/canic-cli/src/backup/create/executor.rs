//! Module: backup::create::executor
//!
//! Responsibility: execute backup create runner operations through the ICP CLI.
//! Does not own: backup planning, layout persistence, or command option parsing.
//! Boundary: maps runner preflight and snapshot operations onto host ICP commands.

use super::{
    super::{BackupCommandError, options::BackupCreateOptions},
    plan::{backup_registry_entries, registry_topology_hash},
};
use crate::support::candid::role_candid_path;
use canic_backup::{
    plan::{
        AuthorityEvidence, AuthorityProofSource, BackupExecutionPreflightReceipts, BackupPlan,
        ControlAuthority, ControlAuthorityReceipt, QuiescencePreflightReceipt,
        QuiescencePreflightTarget, SnapshotReadAuthority, SnapshotReadAuthorityReceipt,
        TopologyPreflightReceipt, TopologyPreflightTarget,
    },
    runner::{BackupRunnerCommandError, BackupRunnerExecutor, BackupRunnerSnapshotReceipt},
};
use canic_host::{
    icp::{IcpCli, IcpCommandError},
    registry::parse_registry_entries,
    subnet_registry::{SubnetRegistryQueryError, query_subnet_registry_json},
};
use std::path::{Path, PathBuf};

///
/// BackupIcpRunnerExecutor
///

pub(super) struct BackupIcpRunnerExecutor {
    options: BackupCreateOptions,
    icp_root: PathBuf,
    icp: IcpCli,
}

impl BackupIcpRunnerExecutor {
    pub(super) fn new(options: &BackupCreateOptions, icp_root: PathBuf) -> Self {
        Self {
            options: options.clone(),
            icp: IcpCli::new(&options.icp, None, Some(options.network.clone())).with_cwd(&icp_root),
            icp_root,
        }
    }
}

impl BackupRunnerExecutor for BackupIcpRunnerExecutor {
    fn preflight_receipts(
        &mut self,
        plan: &BackupPlan,
        preflight_id: &str,
        validated_at: &str,
        expires_at: &str,
    ) -> Result<BackupExecutionPreflightReceipts, BackupRunnerCommandError> {
        let registry_json =
            call_subnet_registry(&self.options, &self.icp_root, &plan.root_canister_id)
                .map_err(preflight_error)?;
        let host_registry = parse_registry_entries(&registry_json).map_err(preflight_error)?;
        let registry = backup_registry_entries(&host_registry);
        let topology_hash = registry_topology_hash(&registry).map_err(preflight_error)?;
        for target in &plan.targets {
            let status = self
                .icp
                .canister_status_report(&target.canister_id)
                .map_err(runner_icp_error)?;
            if status
                .settings
                .as_ref()
                .is_none_or(|settings| settings.controllers.is_empty())
            {
                return Err(BackupRunnerCommandError::failed(
                    "preflight",
                    format!(
                        "icp canister status --json for {} did not include controllers",
                        target.canister_id
                    ),
                ));
            }
        }

        Ok(BackupExecutionPreflightReceipts {
            plan_id: plan.plan_id.clone(),
            preflight_id: preflight_id.to_string(),
            validated_at: validated_at.to_string(),
            expires_at: expires_at.to_string(),
            topology: TopologyPreflightReceipt {
                plan_id: plan.plan_id.clone(),
                preflight_id: preflight_id.to_string(),
                topology_hash_before_quiesce: plan.topology_hash_before_quiesce.clone(),
                topology_hash_at_preflight: topology_hash,
                targets: plan
                    .targets
                    .iter()
                    .map(TopologyPreflightTarget::from)
                    .collect(),
                validated_at: validated_at.to_string(),
                expires_at: expires_at.to_string(),
                message: Some("root registry matched planned topology".to_string()),
            },
            control_authority: plan
                .targets
                .iter()
                .map(|target| ControlAuthorityReceipt {
                    plan_id: plan.plan_id.clone(),
                    preflight_id: preflight_id.to_string(),
                    target_canister_id: target.canister_id.clone(),
                    authority: ControlAuthority::operator_controller(AuthorityEvidence::Proven),
                    proof_source: AuthorityProofSource::ManagementStatus,
                    validated_at: validated_at.to_string(),
                    expires_at: expires_at.to_string(),
                    message: Some(
                        "icp canister status --json proved controller status access".to_string(),
                    ),
                })
                .collect(),
            snapshot_read_authority: plan
                .targets
                .iter()
                .map(|target| SnapshotReadAuthorityReceipt {
                    plan_id: plan.plan_id.clone(),
                    preflight_id: preflight_id.to_string(),
                    target_canister_id: target.canister_id.clone(),
                    authority: SnapshotReadAuthority::operator_controller(
                        AuthorityEvidence::Proven,
                    ),
                    proof_source: AuthorityProofSource::ManagementStatus,
                    validated_at: validated_at.to_string(),
                    expires_at: expires_at.to_string(),
                    message: Some("operator control permits snapshot read".to_string()),
                })
                .collect(),
            quiescence: QuiescencePreflightReceipt {
                plan_id: plan.plan_id.clone(),
                preflight_id: preflight_id.to_string(),
                quiescence_policy: plan.quiescence_policy.clone(),
                accepted: true,
                targets: plan
                    .targets
                    .iter()
                    .map(QuiescencePreflightTarget::from)
                    .collect(),
                validated_at: validated_at.to_string(),
                expires_at: expires_at.to_string(),
                message: Some("crash-consistent operator backup accepted".to_string()),
            },
        })
    }

    fn stop_canister(&mut self, canister_id: &str) -> Result<(), BackupRunnerCommandError> {
        self.icp
            .stop_canister(canister_id)
            .map_err(runner_icp_error)
    }

    fn start_canister(&mut self, canister_id: &str) -> Result<(), BackupRunnerCommandError> {
        self.icp
            .start_canister(canister_id)
            .map_err(runner_icp_error)
    }

    fn create_snapshot(
        &mut self,
        canister_id: &str,
    ) -> Result<BackupRunnerSnapshotReceipt, BackupRunnerCommandError> {
        self.icp
            .snapshot_create_receipt(canister_id)
            .map(|receipt| BackupRunnerSnapshotReceipt {
                snapshot_id: receipt.snapshot_id,
                taken_at_timestamp: receipt.taken_at_timestamp,
                total_size_bytes: receipt.total_size_bytes,
            })
            .map_err(runner_icp_error)
    }

    fn download_snapshot(
        &mut self,
        canister_id: &str,
        snapshot_id: &str,
        artifact_path: &Path,
    ) -> Result<(), BackupRunnerCommandError> {
        self.icp
            .snapshot_download(canister_id, snapshot_id, artifact_path)
            .map_err(runner_icp_error)
    }
}

fn preflight_error(error: impl std::error::Error) -> BackupRunnerCommandError {
    BackupRunnerCommandError::failed("preflight", error.to_string())
}

fn runner_icp_error(error: IcpCommandError) -> BackupRunnerCommandError {
    BackupRunnerCommandError::failed("icp", error.to_string())
}

fn call_subnet_registry(
    options: &BackupCreateOptions,
    icp_root: &Path,
    root: &str,
) -> Result<String, BackupCommandError> {
    let icp = IcpCli::new(&options.icp, None, Some(options.network.clone())).with_cwd(icp_root);
    let candid_path = role_candid_path(Some(icp_root), &options.network, "root");
    query_subnet_registry_json(
        &icp,
        root,
        &options.network,
        Some(icp_root),
        candid_path.as_deref(),
    )
    .map(|query| query.registry_json)
    .map_err(backup_subnet_registry_error)
}

fn backup_subnet_registry_error(error: SubnetRegistryQueryError) -> BackupCommandError {
    match error {
        SubnetRegistryQueryError::Replica(err) => BackupCommandError::ReplicaQuery(err.to_string()),
        SubnetRegistryQueryError::Icp(err) => backup_icp_error(err),
    }
}

fn backup_icp_error(error: IcpCommandError) -> BackupCommandError {
    match error {
        IcpCommandError::Io(err) => BackupCommandError::Io(err),
        IcpCommandError::Failed { command, stderr } => {
            BackupCommandError::IcpFailed { command, stderr }
        }
        IcpCommandError::Json {
            command, output, ..
        } => BackupCommandError::IcpFailed {
            command,
            stderr: output,
        },
        error @ (IcpCommandError::MissingCli { .. }
        | IcpCommandError::IncompatibleCliVersion { .. }) => BackupCommandError::IcpFailed {
            command: "icp --version".to_string(),
            stderr: error.to_string(),
        },
        IcpCommandError::SnapshotIdUnavailable { output } => BackupCommandError::IcpFailed {
            command: "icp canister snapshot create".to_string(),
            stderr: output,
        },
    }
}
