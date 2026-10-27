use super::*;
use canic_contracts::{
    dto::fleet_registry::{FleetComponentSpecEntry, FleetSubnetRootEntry},
    ids::{
        ComponentSpecAdmission, CyclesFundingBudget, FleetBinding, FleetCoordinatorBinding,
        FleetId, FleetKey, FleetRegistryAuthority, FleetSubnetCanisterPoolConfig,
        FleetSubnetRootLimits, FleetSubnetRootReleaseSet, ReleaseBuildId, ReleaseBuildNonce,
        ReleaseSetDigest,
    },
};
use canic_core::shared_support::fleet_admission_policy::compile_installed_fleet_admission_policy;

fn fixture() -> (ComponentTopology, FleetRegistry) {
    let topology = canic_core::bootstrap::parse_config_model(
        r#"
[app]
name = "funding"
[roles.root]
kind = "root"
[roles.worker]
kind = "canister"
package = "worker"
[component_specs.workers]
component_role = "worker"
maximum_instances = 1
"#,
    )
    .unwrap()
    .compile_component_topology()
    .unwrap();
    let fleet = FleetBinding {
        fleet: FleetKey {
            canonical_network_id: canic_contracts::ids::CanonicalNetworkId::ic_mainnet(),
            fleet_id: FleetId::from_generated_bytes([1; 32]),
        },
        app: "funding".into(),
    };
    let registry = FleetRegistry {
        authority: FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: fleet.clone(),
                coordinator_subnet: Principal::from_slice(&[2]).into(),
                coordinator: Principal::from_slice(&[3]),
                recovery_controllers: Vec::new(),
            },
            epoch: 1,
        },
        revision: 1,
        admission: compile_installed_fleet_admission_policy(
            fleet,
            1,
            vec![Principal::from_slice(&[9])],
            vec![],
        )
        .unwrap(),
        component_specs: topology
            .component_specs
            .iter()
            .map(|spec| FleetComponentSpecEntry {
                component_spec: spec.component_spec.clone(),
                spec_hash: spec.spec_hash,
                component_role: spec.component_role.clone(),
                maximum_fleet_instances: spec.maximum_fleet_instances,
            })
            .collect(),
        fleet_subnet_roots: vec![root(&topology)],
        services: vec![],
    };
    (topology, registry)
}

fn root(topology: &ComponentTopology) -> FleetSubnetRootEntry {
    let spec = &topology.component_specs[0];
    let component_admissions = vec![ComponentSpecAdmission {
        component_spec: spec.component_spec.clone(),
        spec_hash: spec.spec_hash,
        maximum_root_instances: 1,
    }];
    let component_topology_digest = topology
        .project_for_admissions(&component_admissions)
        .unwrap()
        .digest()
        .unwrap();
    FleetSubnetRootEntry {
        placement_subnet: Principal::from_slice(&[4; 29]).into(),
        fleet_subnet_root: Principal::from_slice(&[5]),
        component_admissions,
        component_topology_digest,
        active_release_set: FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [6; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([7; 32]),
        },
        limits: FleetSubnetRootLimits {
            maximum_component_instances: 1,
            maximum_registry_bytes: 2_097_152,
            maximum_wasm_store_bytes: 40_000_000,
            maximum_group_placements: 1,
            canister_pool: FleetSubnetCanisterPoolConfig {
                minimum_size: 1,
                maximum_size: 4,
                canister_cycles: 5_000_000_000_000.into(),
                creation_execution_margin: 1_000_000_000_000.into(),
            },
            cycles_funding: CyclesFundingBudget {
                window_secs: 3600,
                maximum_cycles: 10_000_000_000_000.into(),
            },
        },
        funding: crate::test_support::fleet_subnet_root_funding_authority(),
        status: FleetSubnetRootStatus::Active,
    }
}

fn summary(registry: &StartupFundingRegistry) -> FleetSubnetRootCanisterSummary {
    let (root, placement) = registry.roots.first_key_value().unwrap();
    FleetSubnetRootCanisterSummary {
        fleet_registry: FleetRegistryVersion {
            authority: registry.authority.clone(),
            revision: registry.revision,
            content_hash: registry.content_hash,
        },
        placement_subnet: placement.placement_subnet,
        fleet_subnet_root: *root,
        status: FleetSubnetRootStatus::Active,
        infrastructure_canisters: 2,
        component_canisters: 0,
        pooled_canisters: 1,
        total_canisters: 3,
    }
}

#[test]
fn registry_requires_the_observed_coordinator_and_complete_canonical_topology() {
    let (topology, registry) = fixture();
    let coordinator = registry.authority.binding.coordinator;
    let projected = project(coordinator, &topology, registry.clone()).unwrap();
    assert_eq!(projected.authority, registry.authority);
    assert!(
        projected
            .roots
            .contains_key(&registry.fleet_subnet_roots[0].fleet_subnet_root)
    );
    assert_eq!(
        project(Principal::from_slice(&[99]), &topology, registry.clone()),
        Err(StartupUsageUnavailable::AuthorityMismatch),
    );
    let mut variants = vec![registry; 6];
    variants[0].component_specs.clear();
    variants[1].component_specs[0].spec_hash = [99; 32];
    variants[2].revision = 0;
    variants[3].authority.epoch = 0;
    variants[4].admission.policy_digest = [99; 32];
    let duplicate = variants[5].fleet_subnet_roots[0].clone();
    variants[5].fleet_subnet_roots.push(duplicate);
    for variant in variants {
        assert_eq!(
            project(coordinator, &topology, variant),
            Err(StartupUsageUnavailable::AuthorityMismatch)
        );
    }
}

#[test]
fn root_mirror_requires_current_head_placement_and_active_lifecycle() {
    let (topology, registry) = fixture();
    let registry = project(registry.authority.binding.coordinator, &topology, registry).unwrap();
    let summary = summary(&registry);
    let root = summary.fleet_subnet_root;
    assert_eq!(match_root(root, &registry, &summary), Ok(()));
    let mut wrong = summary.clone();
    wrong.fleet_subnet_root = Principal::from_slice(&[99]);
    assert_eq!(
        match_root(root, &registry, &wrong),
        Err(StartupUsageUnavailable::AuthorityMismatch)
    );
    let mut wrong = summary.clone();
    wrong.placement_subnet = Principal::from_slice(&[99]).into();
    assert_eq!(
        match_root(root, &registry, &wrong),
        Err(StartupUsageUnavailable::AuthorityMismatch)
    );
    let mut stale = summary.clone();
    stale.fleet_registry.revision += 1;
    assert_eq!(
        match_root(root, &registry, &stale),
        Err(StartupUsageUnavailable::PolicyTransition)
    );
    for status in [
        FleetSubnetRootStatus::Joining,
        FleetSubnetRootStatus::Draining,
        FleetSubnetRootStatus::Removed,
    ] {
        let mut inactive = summary.clone();
        inactive.status = status;
        assert_eq!(
            match_root(root, &registry, &inactive),
            Err(StartupUsageUnavailable::PolicyTransition)
        );
    }
    let mut absent = registry;
    absent.roots.clear();
    assert_eq!(
        match_root(root, &absent, &summary),
        Err(StartupUsageUnavailable::AuthorityMismatch)
    );
}

#[test]
fn changed_authority_revision_or_content_invalidates_collected_usage() {
    let (topology, registry) = fixture();
    let version = FleetRegistryOps::version(&registry.authority, &topology, &registry).unwrap();
    let projected = project(registry.authority.binding.coordinator, &topology, registry).unwrap();
    assert_eq!(match_head(&projected, &version), Ok(()));
    let mut variants = vec![version; 3];
    variants[0].authority.epoch += 1;
    variants[1].revision += 1;
    variants[2].content_hash = [99; 32];
    for variant in variants {
        assert_eq!(
            match_head(&projected, &variant),
            Err(StartupUsageUnavailable::PolicyTransition)
        );
    }
}

#[cfg(unix)]
#[test]
fn registry_reads_require_authenticated_query_context_before_transport() {
    use std::os::unix::fs::PermissionsExt as _;
    let directory = crate::test_support::temp_dir("funding-registry");
    std::fs::create_dir_all(&directory).unwrap();
    let executable = directory.join("icp");
    std::fs::write(&executable, "#!/bin/sh\ntouch invoked\nexit 1\n").unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    let (topology, registry) = fixture();
    let coordinator = registry.authority.binding.coordinator;
    let projected = project(coordinator, &topology, registry).unwrap();
    let root = summary(&projected).fleet_subnet_root;
    let icp = IcpCli::new(executable.to_str().unwrap(), None).with_cwd(directory.clone());
    assert_eq!(
        observe(&icp, coordinator, &topology),
        Err(StartupUsageUnavailable::ObservationFailed)
    );
    assert_eq!(
        observe_root(&icp, root, &projected),
        Err(StartupUsageUnavailable::ObservationFailed)
    );
    assert!(!directory.join("invoked").exists());
    assert_eq!(icp.remote_call_count(), 0);
    std::fs::remove_dir_all(directory).unwrap();
}
