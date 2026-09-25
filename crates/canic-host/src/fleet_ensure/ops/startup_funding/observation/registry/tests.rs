use super::*;
use canic_core::{
    dto::fleet_registry::{FleetComponentSpecEntry, FleetSubnetRootEntry},
    ids::{
        ComponentSpecAdmission, CyclesFundingBudget, FleetBinding, FleetCoordinatorBinding,
        FleetId, FleetKey, FleetRegistryAuthority, FleetSubnetCanisterPoolConfig,
        FleetSubnetRootLimits, FleetSubnetRootReleaseSet, ReleaseBuildId, ReleaseBuildNonce,
        ReleaseSetDigest,
    },
    shared_support::fleet_admission_policy::compile_installed_fleet_admission_policy,
};

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
            canonical_network_id: canic_core::ids::CanonicalNetworkId::ic_mainnet(),
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
fn registry_transport_is_query_only_and_failed_or_changed_heads_do_not_qualify() {
    use std::os::unix::fs::PermissionsExt as _;
    let directory = crate::test_support::temp_dir("funding-registry");
    std::fs::create_dir_all(&directory).unwrap();
    let executable = directory.join("icp");
    std::fs::write(
        &executable,
        crate::test_support::tool_script(
            r#"#!/bin/sh
case "$*" in
  --version) printf 'icp @ICP_VERSION@\n' ;;
  *canic_coordinator_registry*--query*) cat snapshot.json ;;
  *canic_observability*--query*) cat head.json ;;
  *canic_root_status*--query*) cat root.json ;;
  *) exit 1 ;;
esac
"#,
        ),
    )
    .unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    let candid = directory.join("coordinator.did");
    std::fs::write(
        &candid,
        include_str!("../../../../../../../canic/candid/fleet_coordinator.did"),
    )
    .unwrap();
    let write = |name: &str, bytes: Vec<u8>| {
        std::fs::write(
            directory.join(name),
            serde_json::json!({
                "response_bytes": canic_core::cdk::utils::hash::hex_bytes(&bytes)
            })
            .to_string(),
        )
        .unwrap();
    };
    let (topology, registry) = fixture();
    let coordinator = registry.authority.binding.coordinator;
    let mut version = FleetRegistryOps::version(&registry.authority, &topology, &registry).unwrap();
    write(
        "snapshot.json",
        candid::encode_one(Ok::<_, canic_core::dto::error::Error>(
            CoordinatorRegistryResponse::Registry(registry),
        ))
        .unwrap(),
    );
    let write_head = |version| {
        write(
            "head.json",
            candid::encode_one(Ok::<_, canic_core::dto::error::Error>(
                CoordinatorObservabilityResponse::RegistryVersion(version),
            ))
            .unwrap(),
        );
    };
    let icp = IcpCli::new(executable.to_str().unwrap(), None).with_cwd(directory.clone());
    let observed = observe(&icp, &candid, coordinator, &topology).unwrap();
    let root_summary = summary(&observed);
    let root = root_summary.fleet_subnet_root;
    write(
        "root.json",
        candid::encode_one(Ok::<_, canic_core::dto::error::Error>(
            RootResponse::Inventory(root_summary),
        ))
        .unwrap(),
    );
    // The adapter uses Rust encoding; generated Root sidecar equality is covered separately.
    let root_candid = directory.join("root.did");
    std::fs::write(&root_candid, "service : {}").unwrap();
    assert_eq!(
        observe_root(&icp, &root_candid, root, &observed),
        Ok(StartupRootInventory { workloads: 0 })
    );
    std::fs::remove_file(directory.join("root.json")).unwrap();
    assert_eq!(
        observe_root(&icp, &root_candid, root, &observed),
        Err(StartupUsageUnavailable::ObservationFailed)
    );
    write_head(version.clone());
    assert_eq!(recheck(&icp, &candid, &observed), Ok(()));
    version.revision += 1;
    write_head(version);
    assert_eq!(
        recheck(&icp, &candid, &observed),
        Err(StartupUsageUnavailable::PolicyTransition)
    );
    std::fs::remove_file(directory.join("head.json")).unwrap();
    assert_eq!(
        recheck(&icp, &candid, &observed),
        Err(StartupUsageUnavailable::ObservationFailed)
    );
    std::fs::remove_file(directory.join("snapshot.json")).unwrap();
    assert_eq!(
        observe(&icp, &candid, coordinator, &topology),
        Err(StartupUsageUnavailable::ObservationFailed)
    );
    std::fs::remove_dir_all(directory).unwrap();
}
