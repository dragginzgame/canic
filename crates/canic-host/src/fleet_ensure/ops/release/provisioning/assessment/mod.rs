//! Project bounded provisioning pages into owner-qualified facts; no settlement or dispatch.

use crate::fleet_ensure::view::release::{
    FleetReleaseRootProvisioningView,
    provisioning::{
        ReleaseProvisioningFacts, ReleaseProvisioningIdentity, ReleaseProvisioningOwner,
        ReleaseProvisioningState as State, ReleaseRootProvisioningFacts,
    },
};
use canic_control_plane::dto::root::{
    RootProvisioningReleaseKey as Key, RootProvisioningReleasePhase as Phase,
};
use std::collections::BTreeSet;

pub(in crate::fleet_ensure) fn assessment_facts(
    root: &FleetReleaseRootProvisioningView,
) -> ReleaseRootProvisioningFacts {
    let mut active = BTreeSet::new();
    if let Some(header) = root.pages.first() {
        if let Some(operation_id) = header.active_provisioning {
            active.insert(ReleaseProvisioningIdentity {
                owner: ReleaseProvisioningOwner::Provisioning,
                operation_id,
            });
        }
        if let Some(operation_id) = header.active_directory_synchronization {
            active.insert(ReleaseProvisioningIdentity {
                owner: ReleaseProvisioningOwner::DirectorySynchronization,
                operation_id,
            });
        }
    }
    let operations = root
        .pages
        .iter()
        .filter_map(|page| page.entry.as_ref())
        .map(|entry| {
            let (owner, operation_id) = match entry.key {
                Key::Provisioning(id) => (ReleaseProvisioningOwner::Provisioning, id),
                Key::DirectorySynchronization(id) => {
                    (ReleaseProvisioningOwner::DirectorySynchronization, id)
                }
            };
            ReleaseProvisioningFacts {
                identity: ReleaseProvisioningIdentity {
                    owner,
                    operation_id,
                },
                plan_hash: entry.plan_hash,
                state: match entry.phase {
                    Phase::Accepted => State::Accepted,
                    Phase::Provisioned => State::Provisioned,
                    Phase::Publishing => State::Publishing,
                    Phase::Published => State::Published,
                    Phase::Activating => State::Activating,
                    Phase::RuntimesActive => State::RuntimesActive,
                    Phase::DirectoryPlanned => State::DirectoryPlanned,
                    Phase::DirectorySynchronizing => State::DirectorySynchronizing,
                    Phase::DirectorySynchronized => State::DirectorySynchronized,
                },
                delivery_in_flight: entry.delivery_in_flight,
            }
        })
        .collect();
    ReleaseRootProvisioningFacts {
        root: root.root,
        active,
        operations,
    }
}
