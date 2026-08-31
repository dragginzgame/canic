//! Component-provisioning retry authority projection.
//!
//! Boundary: the active durable intent determines the sole retry authority; this owner neither
//! records failures nor mutates the Coordinator journal.

use super::*;

#[derive(Clone, Copy)]
pub(super) struct FleetComponentProvisioningRetryAuthority {
    pub(super) fleet_subnet_root: Principal,
    pub(super) stage: FleetComponentProvisioningRetryStage,
    pub(super) started_at_ns: u64,
}

pub(super) fn current_component_provisioning_retry_authority(
    state: &FleetComponentProvisioningStateRecord,
) -> Option<FleetComponentProvisioningRetryAuthority> {
    match state {
        FleetComponentProvisioningStateRecord::AcceptingRoots {
            in_flight: Some(intent),
            ..
        } => Some(FleetComponentProvisioningRetryAuthority {
            fleet_subnet_root: intent.fleet_subnet_root,
            stage: FleetComponentProvisioningRetryStage::RootAcceptance,
            started_at_ns: intent.started_at_ns,
        }),
        FleetComponentProvisioningStateRecord::ProvisioningRoots {
            in_flight: Some(intent),
            ..
        } => Some(FleetComponentProvisioningRetryAuthority {
            fleet_subnet_root: intent.fleet_subnet_root,
            stage: FleetComponentProvisioningRetryStage::RootProvisioning,
            started_at_ns: intent.started_at_ns,
        }),
        FleetComponentProvisioningStateRecord::ConfirmingDirectories {
            in_flight: Some(intent),
            ..
        } => Some(FleetComponentProvisioningRetryAuthority {
            fleet_subnet_root: confirmation_intent_root(intent),
            stage: FleetComponentProvisioningRetryStage::DirectoryConfirmation,
            started_at_ns: confirmation_intent_started_at_ns(intent),
        }),
        FleetComponentProvisioningStateRecord::ActivatingRuntimes {
            in_flight: Some(intent),
            ..
        } => Some(FleetComponentProvisioningRetryAuthority {
            fleet_subnet_root: intent.fleet_subnet_root,
            stage: FleetComponentProvisioningRetryStage::RuntimeActivation,
            started_at_ns: intent.started_at_ns,
        }),
        _ => None,
    }
}

pub(super) fn pending_component_provisioning_root_failure(
    record: &FleetComponentProvisioningRecord,
) -> Option<FleetComponentProvisioningRootFailure> {
    let failure = record.last_root_failure?;
    let current = current_component_provisioning_retry_authority(&record.state)?;
    let same_root = failure.fleet_subnet_root == current.fleet_subnet_root;
    let same_stage = failure.stage == current.stage;
    let follows_current_intent = failure.failed_at_ns >= current.started_at_ns;
    let matches_current_retry = [same_root, same_stage, follows_current_intent]
        .into_iter()
        .all(std::convert::identity);
    matches_current_retry.then_some(failure)
}
