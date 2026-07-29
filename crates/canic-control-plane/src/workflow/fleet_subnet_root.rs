//! Module: workflow::fleet_subnet_root
//!
//! Responsibility: project compact live Fleet Subnet Root operator evidence.
//! Does not own: Registry mutation, Component lifecycle effects, or CLI aggregation.
//! Boundary: summaries are emitted only from mutually consistent protected, mirror, Store,
//! runtime, and Component Registry authority.

use crate::{
    ops::{
        component_registry::ComponentRegistryOps, fleet_registry_mirror::FleetRegistryMirrorOps,
        storage::state::subnet::SubnetStateOps,
    },
    view::{
        component_registry::RootComponentRegistryView,
        fleet_registry_mirror::RootFleetRegistryActiveView,
    },
};
use canic_core::{
    api::fleet_activation::FleetActivationApi,
    control_plane_support::{
        error::{InternalError, InternalErrorOrigin},
        ops::{config::ConfigOps, fleet_registry::FleetRegistryOps, ic::IcOps},
    },
    dto::{
        fleet_registry::{FleetRegistryVersion, FleetSubnetRootEntry, FleetSubnetRootStatus},
        fleet_subnet_root::{FleetSubnetRootAuthority, FleetSubnetRootCanisterSummary},
    },
};

/// Return one compact, fail-closed inventory for this active Fleet Subnet Root.
pub fn canister_summary() -> Result<FleetSubnetRootCanisterSummary, InternalError> {
    let authority = FleetActivationApi::root_authority().map_err(InternalError::public)?;
    FleetActivationApi::require_active().map_err(InternalError::public)?;
    let root = IcOps::canister_self();
    if authority.binding.fleet_subnet_root != root {
        return Err(InternalError::invalid_input(
            "protected Fleet Subnet Root authority does not name this Canister",
        ));
    }

    let mirror = FleetRegistryMirrorOps::current()
        .active
        .ok_or_else(|| InternalError::unavailable("root has no active Fleet Registry mirror"))?;
    let (fleet_registry, root_entry) = validated_registry_authority(&authority, root, &mirror)?;
    let component_registry = ComponentRegistryOps::current().ok_or_else(|| {
        InternalError::unavailable("root Component Registry authority has not been prepared")
    })?;
    validate_component_registry(&authority, &fleet_registry, &component_registry)?;

    let store_canisters = u32::try_from(SubnetStateOps::wasm_stores().len()).map_err(|_| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "root-local Wasm Store count exceeds u32",
        )
    })?;
    if store_canisters != 1 {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            format!(
                "active Fleet Subnet Root requires exactly one known local Wasm Store, found {store_canisters}"
            ),
        ));
    }

    summary(
        fleet_registry,
        root_entry,
        &component_registry,
        store_canisters,
    )
}

fn validated_registry_authority(
    authority: &FleetSubnetRootAuthority,
    root: candid::Principal,
    mirror: &RootFleetRegistryActiveView,
) -> Result<(FleetRegistryVersion, FleetSubnetRootEntry), InternalError> {
    let topology = ConfigOps::component_topology()?;
    FleetRegistryOps::validate(
        &authority.binding.authority,
        &topology,
        &mirror.snapshot.registry,
    )?;
    let manifest = FleetRegistryOps::manifest(
        &authority.binding.authority,
        &topology,
        &mirror.snapshot.registry,
    )?;
    let version = FleetRegistryOps::version(
        &authority.binding.authority,
        &topology,
        &mirror.snapshot.registry,
    )?;
    let directory = FleetRegistryOps::active_directory_for_root(
        &authority.binding.authority,
        &topology,
        &mirror.snapshot.registry,
        root,
    )?;
    if mirror.snapshot.manifest != manifest
        || mirror.snapshot.version != version
        || mirror.directory != directory
    {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "active root Registry mirror evidence is not internally consistent",
        ));
    }

    let root_entry = mirror
        .snapshot
        .registry
        .fleet_subnet_roots
        .iter()
        .find(|entry| entry.fleet_subnet_root == root)
        .cloned()
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "active root Registry mirror does not contain this root",
            )
        })?;
    let expected = FleetSubnetRootEntry {
        placement_subnet: authority.binding.placement_subnet,
        fleet_subnet_root: root,
        component_admissions: authority.binding.component_admissions.clone(),
        component_topology_digest: authority.binding.component_topology_digest,
        active_release_set: authority.initial_release_set,
        limits: authority.binding.limits.clone(),
        status: root_entry.status,
    };
    if root_entry != expected || root_entry.status == FleetSubnetRootStatus::Removed {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "active root Registry row differs from protected authority or is Removed",
        ));
    }
    Ok((version, root_entry))
}

fn validate_component_registry(
    authority: &FleetSubnetRootAuthority,
    fleet_registry: &FleetRegistryVersion,
    registry: &RootComponentRegistryView,
) -> Result<(), InternalError> {
    if registry.root != authority.binding
        || &registry.prepared_against_registry != fleet_registry
        || registry.release_set != authority.initial_release_set
    {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component Registry authority differs from the active root Registry mirror",
        ));
    }

    let allocated_canisters = registry
        .reserved_component_instances
        .checked_add(registry.committed_component_instances)
        .and_then(|count| count.checked_add(registry.managed_descendants))
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "Component Registry allocation counters overflow",
            )
        })?;
    if registry.known_created_component_canisters > allocated_canisters {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "known-created Component Canisters exceed allocated Component-tree capacity",
        ));
    }
    Ok(())
}

fn summary(
    fleet_registry: FleetRegistryVersion,
    root_entry: FleetSubnetRootEntry,
    registry: &RootComponentRegistryView,
    store_canisters: u32,
) -> Result<FleetSubnetRootCanisterSummary, InternalError> {
    let infrastructure_canisters = 1_u32.checked_add(store_canisters).ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "root-local infrastructure Canister count overflow",
        )
    })?;
    let managed_canisters = store_canisters
        .checked_add(registry.known_created_component_canisters)
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "root-managed Canister count overflow",
            )
        })?;
    if managed_canisters > root_entry.limits.maximum_managed_canisters {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "known-created Canisters exceed the protected root limit",
        ));
    }
    let total_canisters = infrastructure_canisters
        .checked_add(registry.known_created_component_canisters)
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "Fleet Subnet Root Canister summary total overflow",
            )
        })?;

    Ok(FleetSubnetRootCanisterSummary {
        fleet_registry,
        placement_subnet: root_entry.placement_subnet,
        fleet_subnet_root: root_entry.fleet_subnet_root,
        status: root_entry.status,
        infrastructure_canisters,
        component_canisters: registry.known_created_component_canisters,
        total_canisters,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::{
        cdk::types::Cycles,
        dto::root_store::RootStoreBootstrapRequest,
        ids::{
            AppId, CanonicalNetworkId, ComponentTopologyDigest, CyclesFundingBudget, FleetBinding,
            FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority,
            FleetSubnetRootBinding, FleetSubnetRootLimits, FleetSubnetRootReleaseSet,
            ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
        },
    };

    #[test]
    fn summary_reports_exact_checked_counts_without_member_enumeration() {
        let authority = authority();
        let version = version(&authority);
        let registry = component_registry(&authority, version.clone(), 3, 1, 2, 0);
        validate_component_registry(&authority, &version, &registry)
            .expect("validate Component Registry counters");

        let summary = summary(
            version,
            root_entry(&authority, FleetSubnetRootStatus::Active),
            &registry,
            1,
        )
        .expect("build summary");

        assert_eq!(summary.infrastructure_canisters, 2);
        assert_eq!(summary.component_canisters, 3);
        assert_eq!(summary.total_canisters, 5);
    }

    #[test]
    fn summary_rejects_counter_and_protected_limit_drift() {
        let authority = authority();
        let version = version(&authority);
        let invalid_registry = component_registry(&authority, version.clone(), 4, 1, 2, 0);
        assert!(
            validate_component_registry(&authority, &version, &invalid_registry).is_err(),
            "known-created count must not exceed allocated Component-tree capacity"
        );

        let registry = component_registry(&authority, version.clone(), 3, 1, 2, 0);
        let mut entry = root_entry(&authority, FleetSubnetRootStatus::Active);
        entry.limits.maximum_managed_canisters = 3;
        assert!(
            summary(version, entry, &registry, 1).is_err(),
            "Store plus Component Canisters must not exceed the protected managed limit"
        );
    }

    fn authority() -> FleetSubnetRootAuthority {
        let release_set = FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [8; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
        };
        FleetSubnetRootAuthority {
            binding: FleetSubnetRootBinding {
                authority: FleetRegistryAuthority {
                    binding: FleetCoordinatorBinding {
                        fleet: FleetBinding {
                            fleet: FleetKey {
                                canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                                fleet_id: FleetId::from_generated_bytes([1; 32]),
                            },
                            app: AppId::from("toko"),
                        },
                        coordinator_subnet: SubnetId::from_principal(
                            candid::Principal::from_slice(&[2; 29]),
                        ),
                        coordinator: candid::Principal::from_slice(&[3; 29]),
                    },
                    epoch: 1,
                },
                placement_subnet: SubnetId::from_principal(candid::Principal::from_slice(&[4; 29])),
                fleet_subnet_root: candid::Principal::from_slice(&[5; 29]),
                component_admissions: Vec::new(),
                component_topology_digest: ComponentTopologyDigest::from_bytes([6; 32]),
                limits: FleetSubnetRootLimits {
                    maximum_component_instances: 10,
                    maximum_managed_canisters: 10,
                    maximum_registry_bytes: 1_024,
                    maximum_wasm_store_bytes: 2_048,
                    cycles_funding: CyclesFundingBudget {
                        window_secs: 60,
                        maximum_cycles: Cycles::new(1_000_000),
                    },
                },
            },
            initial_release_set: release_set,
            expected_module_hash: [7; 32],
        }
    }

    fn version(authority: &FleetSubnetRootAuthority) -> FleetRegistryVersion {
        FleetRegistryVersion {
            authority: authority.binding.authority.clone(),
            revision: 4,
            content_hash: [10; 32],
        }
    }

    fn component_registry(
        authority: &FleetSubnetRootAuthority,
        prepared_against_registry: FleetRegistryVersion,
        known_created_component_canisters: u32,
        reserved_component_instances: u32,
        committed_component_instances: u32,
        managed_descendants: u32,
    ) -> RootComponentRegistryView {
        RootComponentRegistryView {
            root: authority.binding.clone(),
            prepared_against_registry,
            release_set: authority.initial_release_set,
            store_bootstrap: RootStoreBootstrapRequest {
                manifest_payload_size_bytes: 128,
            },
            next_allocation_sequence: 4,
            reserved_component_instances,
            committed_component_instances,
            managed_descendants,
            known_created_component_canisters,
            encoded_bytes: 512,
            initial_inventory: None,
        }
    }

    fn root_entry(
        authority: &FleetSubnetRootAuthority,
        status: FleetSubnetRootStatus,
    ) -> FleetSubnetRootEntry {
        FleetSubnetRootEntry {
            placement_subnet: authority.binding.placement_subnet,
            fleet_subnet_root: authority.binding.fleet_subnet_root,
            component_admissions: authority.binding.component_admissions.clone(),
            component_topology_digest: authority.binding.component_topology_digest,
            active_release_set: authority.initial_release_set,
            limits: authority.binding.limits.clone(),
            status,
        }
    }
}
