//! Module: ops::component_registry
//!
//! Responsibility: read and commit root-local Component Registry meta authority.
//! Does not own: Store, Fleet Registry, topology, admission, or lifecycle validation.
//! Boundary: converts stable records into read-only views before workflow use.

use crate::{
    storage::stable::component_registry::{
        RootComponentRegistryCommitError, RootComponentRegistryMetaRecord,
        RootComponentRegistryStore,
    },
    view::component_registry::RootComponentRegistryView,
};
use canic_core::{
    control_plane_support::error::InternalError,
    dto::fleet_registry::FleetRegistryVersion,
    ids::{FleetSubnetRootBinding, FleetSubnetRootReleaseSet},
};

///
/// ComponentRegistryOps
///
/// Single-step root-local Component Registry meta storage operations.
///

pub struct ComponentRegistryOps;

impl ComponentRegistryOps {
    pub(crate) fn current() -> Option<RootComponentRegistryView> {
        RootComponentRegistryStore::export()
            .current
            .map(record_to_view)
    }

    pub(crate) fn prepare(
        root: FleetSubnetRootBinding,
        prepared_against_registry: FleetRegistryVersion,
        release_set: FleetSubnetRootReleaseSet,
    ) -> Result<RootComponentRegistryView, InternalError> {
        let record = RootComponentRegistryMetaRecord {
            root,
            prepared_against_registry,
            release_set,
            next_allocation_sequence: 1,
            reserved_component_instances: 0,
            committed_component_instances: 0,
            managed_descendants: 0,
            encoded_bytes: 0,
        };
        RootComponentRegistryStore::prepare(record.clone()).map_err(|error| match error {
            RootComponentRegistryCommitError::ConflictingState => InternalError::conflict(
                "root Component Registry is already prepared under different authority",
            ),
        })?;
        Ok(record_to_view(record))
    }
}

fn record_to_view(record: RootComponentRegistryMetaRecord) -> RootComponentRegistryView {
    RootComponentRegistryView {
        root: record.root,
        prepared_against_registry: record.prepared_against_registry,
        release_set: record.release_set,
        next_allocation_sequence: record.next_allocation_sequence,
        reserved_component_instances: record.reserved_component_instances,
        committed_component_instances: record.committed_component_instances,
        managed_descendants: record.managed_descendants,
        encoded_bytes: record.encoded_bytes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::stable::component_registry::RootComponentRegistryData;
    use canic_core::{
        cdk::types::Cycles,
        dto::fleet_registry::FleetRegistryVersion,
        ids::{
            AppId, CanonicalNetworkId, ComponentSpecAdmission, ComponentTopologyDigest,
            CyclesFundingBudget, FleetBinding, FleetCoordinatorBinding, FleetId, FleetKey,
            FleetRegistryAuthority, FleetSubnetRootLimits, ReleaseBuildId, ReleaseBuildNonce,
            ReleaseSetDigest, SubnetId,
        },
    };

    #[test]
    fn preparation_is_exact_idempotent_and_conflict_closed() {
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
        let root = root_binding();
        let version = FleetRegistryVersion {
            authority: root.authority.clone(),
            revision: 4,
            content_hash: [5; 32],
        };
        let release_set = FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [8; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
        };

        let prepared = ComponentRegistryOps::prepare(root.clone(), version.clone(), release_set)
            .expect("prepare");
        let repeated =
            ComponentRegistryOps::prepare(root.clone(), version, release_set).expect("exact retry");

        assert_eq!(prepared, repeated);
        assert_eq!(prepared.next_allocation_sequence, 1);
        assert_eq!(prepared.reserved_component_instances, 0);
        assert_eq!(prepared.committed_component_instances, 0);
        assert_eq!(prepared.managed_descendants, 0);
        assert_eq!(prepared.encoded_bytes, 0);

        let mut conflicting = root;
        conflicting.limits.maximum_component_instances += 1;
        assert!(
            ComponentRegistryOps::prepare(
                conflicting,
                repeated.prepared_against_registry,
                release_set,
            )
            .is_err()
        );
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
    }

    fn root_binding() -> FleetSubnetRootBinding {
        let coordinator_subnet = SubnetId::from_principal(candid::Principal::from_slice(&[2; 29]));
        FleetSubnetRootBinding {
            authority: FleetRegistryAuthority {
                binding: FleetCoordinatorBinding {
                    fleet: FleetBinding {
                        fleet: FleetKey {
                            canonical_network_id: CanonicalNetworkId::public_ic(),
                            fleet_id: FleetId::from_generated_bytes([1; 32]),
                        },
                        app: AppId::from("toko"),
                    },
                    coordinator_subnet,
                    coordinator: candid::Principal::from_slice(&[3; 29]),
                },
                epoch: 1,
            },
            placement_subnet: SubnetId::from_principal(candid::Principal::from_slice(&[4; 29])),
            fleet_subnet_root: candid::Principal::from_slice(&[5; 29]),
            component_admissions: vec![ComponentSpecAdmission {
                component_spec: "projects".parse().expect("Component Spec"),
                spec_hash: [6; 32],
                maximum_root_instances: 10,
            }],
            component_topology_digest: ComponentTopologyDigest::from_bytes([7; 32]),
            limits: FleetSubnetRootLimits {
                maximum_component_instances: 10,
                maximum_managed_canisters: 20_000,
                maximum_registry_bytes: 16_777_216,
                maximum_wasm_store_bytes: 268_435_456,
                cycles_funding: CyclesFundingBudget {
                    window_secs: 3_600,
                    maximum_cycles: Cycles::new(1_000_000_000_000),
                },
            },
        }
    }
}
