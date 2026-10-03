//! Module: ops::component_provisioning::release
//!
//! Responsibility: discover retained provisioning owners through bounded journal reads.
//! Does not own: admission, resumption, settlement or journal mutation.
//! Boundary: project exact storage identities for the controller-authenticated status query.

use crate::{
    dto::root::{
        RootProvisioningReleaseEntry, RootProvisioningReleaseKey,
        RootProvisioningReleasePhase as Phase, RootProvisioningReleaseResponse,
    },
    ops::component_provisioning::{RootComponentProvisioningOps, failure_response, failure_view},
    storage::stable::component_provisioning::{
        RootComponentDirectorySynchronizationStateRecord, RootComponentOperationKey,
        RootComponentOperationRecord, RootComponentProvisioningStateRecordPhase,
        RootComponentProvisioningStore,
    },
    view::provisioning_release::RootComponentOperationEntryView,
};
use candid::Principal;
use canic_core::control_plane_support::error::InternalError;

impl RootComponentProvisioningOps {
    /// Read at most one bounded record and two keys; never scan unrelated operation bodies.
    pub(crate) fn release_status(
        root: Principal,
        start_after: Option<RootProvisioningReleaseKey>,
    ) -> Result<RootProvisioningReleaseResponse, InternalError> {
        let page = RootComponentProvisioningStore::release_page(start_after.map(key_to_record));
        let state = RootComponentProvisioningStore::state();
        let entry = page.entry.map(|entry| project(root, entry)).transpose()?;
        let next_after = entry
            .as_ref()
            .filter(|_| page.has_more)
            .map(|entry| entry.key);
        Ok(RootProvisioningReleaseResponse {
            root,
            active_provisioning: state.active_operation_id,
            active_directory_synchronization: state.active_directory_synchronization_operation_id,
            entry,
            next_after,
        })
    }
}

const fn key_to_record(key: RootProvisioningReleaseKey) -> RootComponentOperationKey {
    match key {
        RootProvisioningReleaseKey::Provisioning(id) => RootComponentOperationKey::Provisioning(id),
        RootProvisioningReleaseKey::DirectorySynchronization(id) => {
            RootComponentOperationKey::DirectorySynchronization(id)
        }
    }
}

fn project(
    root: Principal,
    entry: RootComponentOperationEntryView,
) -> Result<RootProvisioningReleaseEntry, InternalError> {
    let response = match (entry.key, entry.record) {
        (
            RootComponentOperationKey::Provisioning(id),
            RootComponentOperationRecord::Provisioning(record),
        ) => {
            if id != record.operation_id || record.batch.root.fleet_subnet_root != root {
                return Err(InternalError::conflict());
            }
            let (phase, delivery_in_flight) = match record.state {
                RootComponentProvisioningStateRecordPhase::Accepted { .. } => {
                    (Phase::Accepted, None)
                }
                RootComponentProvisioningStateRecordPhase::Provisioned { .. } => {
                    (Phase::Provisioned, None)
                }
                RootComponentProvisioningStateRecordPhase::Publishing { in_flight, .. } => (
                    Phase::Publishing,
                    in_flight.map(|intent| intent.canister_id),
                ),
                RootComponentProvisioningStateRecordPhase::Published { .. } => {
                    (Phase::Published, None)
                }
                RootComponentProvisioningStateRecordPhase::Activating { .. } => {
                    (Phase::Activating, None)
                }
                RootComponentProvisioningStateRecordPhase::RuntimesActive { .. } => {
                    (Phase::RuntimesActive, None)
                }
            };
            RootProvisioningReleaseEntry {
                key: RootProvisioningReleaseKey::Provisioning(id),
                plan_hash: record.plan_hash,
                phase,
                delivery_in_flight,
                last_failure: record
                    .last_failure
                    .map(|failure| failure_response(failure_view(failure))),
            }
        }
        (
            RootComponentOperationKey::DirectorySynchronization(id),
            RootComponentOperationRecord::DirectorySynchronization(record),
        ) => {
            if id != record.operation_id || record.fleet_subnet_root != root {
                return Err(InternalError::conflict());
            }
            let (phase, delivery_in_flight) = match record.state {
                RootComponentDirectorySynchronizationStateRecord::Planned { .. } => {
                    (Phase::DirectoryPlanned, None)
                }
                RootComponentDirectorySynchronizationStateRecord::Synchronizing {
                    in_flight,
                    ..
                } => (
                    Phase::DirectorySynchronizing,
                    in_flight.map(|intent| intent.canister_id),
                ),
                RootComponentDirectorySynchronizationStateRecord::Synchronized { .. } => {
                    (Phase::DirectorySynchronized, None)
                }
            };
            RootProvisioningReleaseEntry {
                key: RootProvisioningReleaseKey::DirectorySynchronization(id),
                plan_hash: record.plan_hash,
                phase,
                delivery_in_flight,
                last_failure: None,
            }
        }
        _ => return Err(InternalError::conflict()),
    };
    Ok(response)
}
