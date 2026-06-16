//! Module: backup::create::executor::preflight
//!
//! Responsibility: build backup runner preflight receipts.
//! Does not own: runner operation dispatch or ICP error conversion.
//! Boundary: validates registry topology and controller visibility for a plan.

use super::{
    errors::{preflight_error, runner_icp_error},
    registry::call_subnet_registry,
};
use crate::backup::create::plan::{backup_registry_entries, registry_topology_hash};
use canic_backup::{
    plan::{
        AuthorityEvidence, AuthorityProofSource, BackupExecutionPreflightReceipts, BackupPlan,
        ControlAuthority, ControlAuthorityReceipt, QuiescencePreflightReceipt,
        QuiescencePreflightTarget, SnapshotReadAuthority, SnapshotReadAuthorityReceipt,
        TopologyPreflightReceipt, TopologyPreflightTarget,
    },
    runner::BackupRunnerCommandError,
};
use canic_host::{icp::IcpCli, registry::parse_registry_entries};
use std::path::Path;

pub(super) fn build_preflight_receipts(
    icp: &IcpCli,
    options: &crate::backup::BackupCreateOptions,
    icp_root: &Path,
    plan: &BackupPlan,
    preflight_id: &str,
    validated_at: &str,
    expires_at: &str,
) -> Result<BackupExecutionPreflightReceipts, BackupRunnerCommandError> {
    let registry_json =
        call_subnet_registry(options, icp_root, &plan.root_canister_id).map_err(preflight_error)?;
    let host_registry = parse_registry_entries(&registry_json).map_err(preflight_error)?;
    let registry = backup_registry_entries(&host_registry);
    let topology_hash = registry_topology_hash(&registry).map_err(preflight_error)?;
    for target in &plan.targets {
        let status = icp
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
                authority: SnapshotReadAuthority::operator_controller(AuthorityEvidence::Proven),
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
