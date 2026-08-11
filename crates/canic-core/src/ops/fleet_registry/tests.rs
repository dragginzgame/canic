//! Module: ops::fleet_registry::tests
//!
//! Responsibility: freeze canonical Fleet Registry genesis and root-row invariants.
//! Does not own: persistence, synchronization, or lifecycle-effect coverage.
//! Boundary: exercises validation and encoding through one exact compiled Component Topology.

use super::*;
use crate::{
    bootstrap::parse_config_model,
    config::{FleetServiceMemberPurpose, FleetServicePlacementPolicy},
    dto::fleet_registry::{
        FleetDirectoryService, FleetDirectoryServiceComponent, FleetServiceBinding,
        FleetServiceComponentBinding, FleetServiceMode, FleetSubnetRootEntry,
        FleetSubnetRootStatus,
    },
    ids::{
        AppId, CanonicalNetworkId, ComponentGroupMemberPath, ComponentGroupPlacementId,
        ComponentInstanceId, ComponentSpecAdmission, CyclesFundingBudget, FleetBinding,
        FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority, FleetSubnetRootLimits,
        FleetSubnetRootReleaseSet, ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
    },
};
use candid::Principal;

fn topology() -> ComponentTopology {
    parse_config_model(
        r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.alpha]
kind = "canister"
package = "alpha"

[roles.beta]
kind = "canister"
package = "beta"

[component_specs.alpha]
component_role = "alpha"
maximum_instances = 3

[component_specs.beta]
component_role = "beta"
maximum_instances = 2
"#,
    )
    .expect("valid config")
    .compile_component_topology()
    .expect("Component Topology")
}

fn authority() -> FleetRegistryAuthority {
    FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                    fleet_id: FleetId::from_generated_bytes([7; 32]),
                },
                app: AppId::from("demo"),
            },
            coordinator_subnet: subnet(2),
            coordinator: principal(3),
        },
        epoch: 1,
    }
}

fn subnet(byte: u8) -> SubnetId {
    SubnetId::from_principal(principal(byte))
}

fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

fn release_set(byte: u8) -> FleetSubnetRootReleaseSet {
    release_set_for_build(9, byte)
}

fn release_set_for_build(build_byte: u8, manifest_byte: u8) -> FleetSubnetRootReleaseSet {
    FleetSubnetRootReleaseSet {
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
            [build_byte; 32],
        )),
        manifest_digest: ReleaseSetDigest::from_bytes([manifest_byte; 32]),
    }
}

fn limits() -> FleetSubnetRootLimits {
    FleetSubnetRootLimits {
        maximum_component_instances: 4,
        maximum_registry_bytes: 2_097_152,
        maximum_wasm_store_bytes: 40_000_000,
        maximum_group_placements: 16,
        canister_pool: crate::ids::FleetSubnetCanisterPoolConfig {
            minimum_size: 1,
            maximum_size: 10,
            canister_cycles: crate::cdk::types::Cycles::new(5_000_000_000_000),
        },
        cycles_funding: CyclesFundingBudget {
            window_secs: 3_600,
            maximum_cycles: crate::cdk::types::Cycles::new(10_000_000_000_000),
        },
    }
}

fn root(
    topology: &ComponentTopology,
    subnet_byte: u8,
    root_byte: u8,
    admissions: &[(&str, u32)],
) -> FleetSubnetRootEntry {
    let component_admissions = admissions
        .iter()
        .map(|(component_spec, maximum_root_instances)| {
            let component_spec = component_spec.parse().expect("canonical Component Spec ID");
            let spec = topology.get(&component_spec).expect("known Component Spec");
            ComponentSpecAdmission {
                component_spec,
                spec_hash: spec.spec_hash,
                maximum_root_instances: *maximum_root_instances,
            }
        })
        .collect::<Vec<_>>();
    let projection = topology
        .project_for_admissions(&component_admissions)
        .expect("root projection");

    FleetSubnetRootEntry {
        placement_subnet: subnet(subnet_byte),
        fleet_subnet_root: principal(root_byte),
        component_admissions,
        component_topology_digest: projection.digest().expect("topology digest"),
        active_release_set: release_set(root_byte),
        limits: limits(),
        status: FleetSubnetRootStatus::Joining,
    }
}

fn active_registry(topology: &ComponentTopology) -> FleetRegistry {
    let authority = authority();
    let mut joining =
        validation::compile_genesis(&AppId::from("demo"), authority.clone(), topology)
            .expect("valid genesis Registry");
    joining.fleet_subnet_roots = vec![
        root(topology, 5, 6, &[("alpha", 1)]),
        root(topology, 7, 8, &[("alpha", 2), ("beta", 2)]),
    ];
    joining.revision = 3;
    FleetRegistryOps::compile_active(&authority, topology, &joining).expect("active Registry")
}

fn member(
    purpose: FleetServiceMemberPurpose,
    component_byte: u8,
    root_byte: u8,
    canister_byte: u8,
    deployment: &str,
    ordinal: u32,
    member_path: &[&str],
) -> FleetServiceComponentBinding {
    FleetServiceComponentBinding {
        member_purpose: purpose,
        component: ComponentInstanceId::from_generated_bytes([component_byte; 32]),
        fleet_subnet_root: principal(root_byte),
        canister_id: principal(canister_byte),
        group_placement: ComponentGroupPlacementId {
            deployment: deployment.parse().expect("deployment ID"),
            ordinal,
        },
        member_path: ComponentGroupMemberPath::try_from(
            member_path
                .iter()
                .map(|segment| segment.parse().expect("member ID"))
                .collect::<Vec<_>>(),
        )
        .expect("member path"),
    }
}

fn authority_replica_service() -> FleetServiceBinding {
    FleetServiceBinding {
        service: "database".parse().expect("service ID"),
        role: "alpha".parse().expect("role"),
        component_spec: "alpha".parse().expect("Component Spec ID"),
        mode: FleetServiceMode::AuthorityReplica,
        placement: FleetServicePlacementPolicy {
            maximum_members_per_root: 1,
            minimum_distinct_roots: 2,
        },
        members: vec![
            member(
                FleetServiceMemberPurpose::Authority,
                20,
                6,
                30,
                "primary",
                1,
                &["database"],
            ),
            member(
                FleetServiceMemberPurpose::Replica,
                21,
                8,
                31,
                "replica",
                1,
                &["database"],
            ),
        ],
    }
}

fn active_pool_service() -> FleetServiceBinding {
    FleetServiceBinding {
        service: "workers".parse().expect("service ID"),
        role: "beta".parse().expect("role"),
        component_spec: "beta".parse().expect("Component Spec ID"),
        mode: FleetServiceMode::ActivePool,
        placement: FleetServicePlacementPolicy {
            maximum_members_per_root: 1,
            minimum_distinct_roots: 1,
        },
        members: vec![member(
            FleetServiceMemberPurpose::PoolMember,
            22,
            8,
            32,
            "workers",
            1,
            &["worker"],
        )],
    }
}

#[test]
fn genesis_is_revision_one_with_complete_specs_and_no_roots() {
    let topology = topology();
    let registry = validation::compile_genesis(&AppId::from("demo"), authority(), &topology)
        .expect("valid genesis Registry");

    assert_eq!(registry.revision, 1);
    assert_eq!(registry.component_specs.len(), 2);
    assert!(registry.fleet_subnet_roots.is_empty());
    assert!(registry.services.is_empty());
    assert_eq!(
        registry
            .component_specs
            .iter()
            .map(|entry| entry.component_spec.as_str())
            .collect::<Vec<_>>(),
        vec!["alpha", "beta"]
    );

    let bytes = candid::encode_one(&registry).expect("encode Fleet Registry Candid");
    let decoded: FleetRegistry = candid::decode_one(&bytes).expect("decode Fleet Registry Candid");
    assert_eq!(decoded, registry);
}

#[test]
fn initial_services_publish_as_one_canonical_registry_revision() {
    let topology = topology();
    let active = active_registry(&topology);
    let services = vec![authority_replica_service()];

    let published = FleetRegistryOps::compile_initial_services(
        &active.authority,
        &topology,
        &active,
        services.clone(),
    )
    .expect("publish complete service set");

    assert_eq!(published.revision, active.revision + 1);
    assert_eq!(published.services, services);
    assert_eq!(published.fleet_subnet_roots, active.fleet_subnet_roots);
    assert_ne!(
        FleetRegistryOps::version(&active.authority, &topology, &active)
            .expect("active version")
            .content_hash,
        FleetRegistryOps::version(&published.authority, &topology, &published)
            .expect("published version")
            .content_hash,
    );
    assert_eq!(
        crate::cdk::utils::hash::hex_bytes(
            FleetRegistryOps::version(&published.authority, &topology, &published)
                .expect("published version")
                .content_hash,
        ),
        "98adff7973bce47c3ad807101c49c07a3574498c5f8104c318fdcd90a3b0118b"
    );

    std::assert_matches!(
        FleetRegistryOps::compile_initial_services(
            &published.authority,
            &topology,
            &published,
            published.services.clone(),
        ),
        Err(_)
    );
    std::assert_matches!(
        FleetRegistryOps::compile_initial_services(
            &active.authority,
            &topology,
            &active,
            Vec::new(),
        ),
        Err(_)
    );
}

#[test]
fn scale_out_appends_complete_replica_set_in_one_registry_revision() {
    let topology = topology();
    let active = active_registry(&topology);
    let mut initial_service = authority_replica_service();
    initial_service.placement.maximum_members_per_root = 2;
    let published = FleetRegistryOps::compile_initial_services(
        &active.authority,
        &topology,
        &active,
        vec![initial_service],
    )
    .expect("publish initial service");
    let mut next_services = published.services.clone();
    next_services[0].members.push(member(
        FleetServiceMemberPurpose::Replica,
        23,
        8,
        33,
        "replica",
        2,
        &["database"],
    ));

    let appended = FleetRegistryOps::compile_service_additions(
        &published.authority,
        &topology,
        &published,
        next_services.clone(),
    )
    .expect("append complete scale-out member set");

    assert_eq!(appended.revision, published.revision + 1);
    assert_eq!(appended.services, next_services);
    assert_eq!(appended.services[0].members.len(), 3);
    assert_eq!(
        FleetRegistryOps::affected_existing_service_components(
            &published,
            &appended,
            principal(6),
        )
        .expect("derive existing Authority root member"),
        vec![ComponentInstanceId::from_generated_bytes([20; 32])]
    );
    assert_eq!(
        FleetRegistryOps::affected_existing_service_components(
            &published,
            &appended,
            principal(8),
        )
        .expect("derive existing Replica root member"),
        vec![ComponentInstanceId::from_generated_bytes([21; 32])]
    );
    assert!(
        FleetRegistryOps::affected_existing_service_components(
            &published,
            &appended,
            principal(10),
        )
        .expect("unrelated root has no affected members")
        .is_empty()
    );
    std::assert_matches!(
        FleetRegistryOps::compile_service_additions(
            &appended.authority,
            &topology,
            &appended,
            appended.services.clone(),
        ),
        Err(_)
    );
}

#[test]
fn scale_out_service_append_rejects_authority_replacement_and_removal() {
    let topology = topology();
    let active = active_registry(&topology);
    let mut initial_service = authority_replica_service();
    initial_service.placement.maximum_members_per_root = 2;
    let published = FleetRegistryOps::compile_initial_services(
        &active.authority,
        &topology,
        &active,
        vec![initial_service],
    )
    .expect("publish initial service");

    let mut changed_member = published.services.clone();
    changed_member[0].members[0].canister_id = principal(99);
    std::assert_matches!(
        FleetRegistryOps::compile_service_additions(
            &published.authority,
            &topology,
            &published,
            changed_member,
        ),
        Err(_)
    );

    let mut removed_member = published.services.clone();
    removed_member[0].members.pop();
    std::assert_matches!(
        FleetRegistryOps::compile_service_additions(
            &published.authority,
            &topology,
            &published,
            removed_member,
        ),
        Err(_)
    );

    let mut added_authority = published.services.clone();
    added_authority[0].members.insert(
        1,
        member(
            FleetServiceMemberPurpose::Authority,
            24,
            8,
            34,
            "primary",
            2,
            &["database"],
        ),
    );
    std::assert_matches!(
        FleetRegistryOps::compile_service_additions(
            &published.authority,
            &topology,
            &published,
            added_authority,
        ),
        Err(_)
    );
}

#[test]
fn service_registry_validation_rejects_nearest_authority_substitutions() {
    let topology = topology();
    let active = active_registry(&topology);
    let valid = authority_replica_service();

    let mut cases = Vec::<FleetServiceBinding>::new();
    let mut noncanonical = valid.clone();
    noncanonical.members.reverse();
    cases.push(noncanonical);
    let mut duplicate_component = valid.clone();
    duplicate_component.members[1].component = duplicate_component.members[0].component;
    cases.push(duplicate_component);
    let mut duplicate_canister = valid.clone();
    duplicate_canister.members[1].canister_id = duplicate_canister.members[0].canister_id;
    cases.push(duplicate_canister);
    let mut wrong_root = valid.clone();
    wrong_root.members[1].fleet_subnet_root = principal(99);
    cases.push(wrong_root);
    let mut wrong_purpose = valid.clone();
    wrong_purpose.members[1].member_purpose = FleetServiceMemberPurpose::PoolMember;
    cases.push(wrong_purpose);
    let mut wrong_spec = valid.clone();
    wrong_spec.component_spec = "beta".parse().expect("Component Spec ID");
    cases.push(wrong_spec);
    let mut excessive_density = valid;
    excessive_density.placement.maximum_members_per_root = 0;
    cases.push(excessive_density);

    for service in cases {
        let mut registry = active.clone();
        registry.services = vec![service];
        assert!(
            validation::validate(&registry.authority, &topology, &registry).is_err(),
            "invalid service authority was accepted: {:?}",
            registry.services
        );
    }
}

#[test]
fn service_registry_rejects_cross_service_identity_reuse_and_noncanonical_order() {
    let topology = topology();
    let active = active_registry(&topology);
    let authority_replica = authority_replica_service();
    let active_pool = active_pool_service();

    let mut valid = active.clone();
    valid.services = vec![authority_replica.clone(), active_pool.clone()];
    validation::validate(&valid.authority, &topology, &valid).expect("canonical distinct services");

    let mut noncanonical = active.clone();
    noncanonical.services = vec![active_pool.clone(), authority_replica.clone()];
    std::assert_matches!(
        validation::validate(&noncanonical.authority, &topology, &noncanonical),
        Err(FleetRegistryOpsError::NonCanonicalFleetServiceOrder)
    );

    let mut duplicate_component = active_pool.clone();
    duplicate_component.members[0].component = authority_replica.members[0].component;
    let mut duplicate_registry = active.clone();
    duplicate_registry.services = vec![authority_replica.clone(), duplicate_component];
    std::assert_matches!(
        validation::validate(
            &duplicate_registry.authority,
            &topology,
            &duplicate_registry,
        ),
        Err(FleetRegistryOpsError::DuplicateFleetServiceComponent { .. })
    );

    let mut duplicate_canister = active_pool;
    duplicate_canister.members[0].canister_id = authority_replica.members[0].canister_id;
    let mut duplicate_registry = active;
    duplicate_registry.services = vec![authority_replica, duplicate_canister];
    std::assert_matches!(
        validation::validate(
            &duplicate_registry.authority,
            &topology,
            &duplicate_registry,
        ),
        Err(FleetRegistryOpsError::DuplicateFleetServiceCanister { .. })
    );
}

#[test]
fn canonical_registry_manifest_and_version_are_digest_stable() {
    let topology = topology();
    let mut registry = validation::compile_genesis(&AppId::from("demo"), authority(), &topology)
        .expect("valid genesis Registry");
    registry.fleet_subnet_roots = vec![
        root(&topology, 5, 6, &[("alpha", 1)]),
        root(&topology, 7, 8, &[("alpha", 2), ("beta", 2)]),
    ];

    let bytes = canonical_bytes(&registry.authority, &topology, &registry)
        .expect("canonical Registry bytes");
    let manifest = FleetRegistryOps::manifest(&registry.authority, &topology, &registry)
        .expect("Registry manifest");
    let version = FleetRegistryOps::version(&registry.authority, &topology, &registry)
        .expect("Registry version");
    let expected_hash: [u8; 32] = Sha256::digest(&bytes).into();

    assert_eq!(manifest.byte_length, bytes.len() as u64);
    assert_eq!(manifest.content_hash, expected_hash);
    assert_eq!(version.authority, registry.authority);
    assert_eq!(version.revision, registry.revision);
    assert_eq!(version.content_hash, manifest.content_hash);
    assert_eq!(
        crate::cdk::utils::hash::hex_bytes(manifest.content_hash),
        "0eee880efb941ed2d3391b53fa8c7c415ffbd600bde63807fa5a681c1ed0f5bc"
    );
}

#[test]
fn registry_rejects_spec_drift_and_noncanonical_roots() {
    let topology = topology();
    let mut registry = validation::compile_genesis(&AppId::from("demo"), authority(), &topology)
        .expect("valid genesis Registry");
    registry.component_specs[0].maximum_fleet_instances += 1;
    std::assert_matches!(
        validation::validate(&registry.authority, &topology, &registry),
        Err(FleetRegistryOpsError::FleetComponentSpecMismatch { .. })
    );

    let mut registry = validation::compile_genesis(&AppId::from("demo"), authority(), &topology)
        .expect("valid genesis Registry");
    registry.fleet_subnet_roots = vec![
        root(&topology, 7, 8, &[("beta", 1)]),
        root(&topology, 5, 6, &[("alpha", 1)]),
    ];
    std::assert_matches!(
        validation::validate(&registry.authority, &topology, &registry),
        Err(FleetRegistryOpsError::NonCanonicalFleetSubnetRootOrder)
    );
}

#[test]
fn registry_allows_partial_joining_admissions_but_rejects_fleet_excess() {
    let topology = topology();
    let mut partial = validation::compile_genesis(&AppId::from("demo"), authority(), &topology)
        .expect("valid genesis Registry");
    partial.fleet_subnet_roots = vec![root(&topology, 5, 6, &[("alpha", 1)])];
    validation::validate(&partial.authority, &topology, &partial)
        .expect("a joining Registry need not yet admit every Component Spec");

    let mut excessive = validation::compile_genesis(&AppId::from("demo"), authority(), &topology)
        .expect("valid genesis Registry");
    excessive.fleet_subnet_roots = vec![
        root(&topology, 5, 6, &[("alpha", 2)]),
        root(&topology, 7, 8, &[("alpha", 2)]),
    ];
    std::assert_matches!(
        validation::validate(&excessive.authority, &topology, &excessive),
        Err(FleetRegistryOpsError::FleetAdmissionsExceedMaximum {
            admitted: 4,
            maximum_fleet_instances: 3,
            ..
        })
    );
}

#[test]
fn activation_atomically_transitions_one_nonempty_all_joining_snapshot() {
    let topology = topology();
    let authority = authority();
    let mut joining =
        validation::compile_genesis(&AppId::from("demo"), authority.clone(), &topology)
            .expect("valid genesis Registry");
    joining.fleet_subnet_roots = vec![
        root(&topology, 5, 6, &[("alpha", 1)]),
        root(&topology, 7, 8, &[("alpha", 2), ("beta", 2)]),
    ];
    joining.revision = 3;

    let active = FleetRegistryOps::compile_active(&authority, &topology, &joining)
        .expect("activate complete root set");

    assert_eq!(active.revision, 4);
    assert!(
        active
            .fleet_subnet_roots
            .iter()
            .all(|entry| entry.status == FleetSubnetRootStatus::Active)
    );
    for (before, after) in joining
        .fleet_subnet_roots
        .iter()
        .zip(&active.fleet_subnet_roots)
    {
        assert_eq!(before.placement_subnet, after.placement_subnet);
        assert_eq!(before.fleet_subnet_root, after.fleet_subnet_root);
        assert_eq!(before.component_admissions, after.component_admissions);
        assert_eq!(
            before.component_topology_digest,
            after.component_topology_digest
        );
        assert_eq!(before.active_release_set, after.active_release_set);
        assert_eq!(before.limits, after.limits);
    }
}

#[test]
fn draining_and_removal_transition_only_one_exact_root() {
    let topology = topology();
    let authority = authority();
    let mut joining =
        validation::compile_genesis(&AppId::from("demo"), authority.clone(), &topology)
            .expect("valid genesis Registry");
    joining.fleet_subnet_roots = vec![
        root(&topology, 5, 6, &[("alpha", 1)]),
        root(&topology, 7, 8, &[("alpha", 2), ("beta", 2)]),
    ];
    joining.revision = 3;
    let active = FleetRegistryOps::compile_active(&authority, &topology, &joining)
        .expect("activate complete root set");

    let draining = FleetRegistryOps::compile_draining(&authority, &topology, &active, principal(6))
        .expect("drain one active root");

    assert_eq!(draining.revision, active.revision + 1);
    assert_eq!(
        draining
            .fleet_subnet_roots
            .iter()
            .map(|entry| entry.status)
            .collect::<Vec<_>>(),
        vec![
            FleetSubnetRootStatus::Draining,
            FleetSubnetRootStatus::Active
        ]
    );
    let mut expected = active.clone();
    expected.revision += 1;
    expected.fleet_subnet_roots[0].status = FleetSubnetRootStatus::Draining;
    assert_eq!(draining, expected);
    assert!(
        FleetRegistryOps::compile_draining(&authority, &topology, &draining, principal(6)).is_err()
    );
    assert!(
        FleetRegistryOps::compile_draining(&authority, &topology, &active, principal(9)).is_err()
    );

    let removed = FleetRegistryOps::compile_removed(&authority, &topology, &draining, principal(6))
        .expect("remove one draining root");
    assert_eq!(removed.revision, draining.revision + 1);
    assert_eq!(
        removed
            .fleet_subnet_roots
            .iter()
            .map(|entry| entry.status)
            .collect::<Vec<_>>(),
        vec![
            FleetSubnetRootStatus::Removed,
            FleetSubnetRootStatus::Active
        ]
    );
    assert!(
        FleetRegistryOps::compile_removed(&authority, &topology, &removed, principal(6)).is_err()
    );
    assert!(
        FleetRegistryOps::compile_removed(&authority, &topology, &active, principal(6)).is_err()
    );
    assert!(
        FleetRegistryOps::compile_removed(&authority, &topology, &draining, principal(9)).is_err()
    );
}

#[test]
fn directory_is_an_exact_root_sourced_mixed_lifecycle_projection() {
    let topology = topology();
    let authority = authority();
    let mut joining =
        validation::compile_genesis(&AppId::from("demo"), authority.clone(), &topology)
            .expect("valid genesis Registry");
    joining.fleet_subnet_roots = vec![
        root(&topology, 5, 6, &[("alpha", 1)]),
        root(&topology, 7, 8, &[("alpha", 2), ("beta", 2)]),
    ];
    joining.revision = 3;
    let mut published =
        FleetRegistryOps::compile_active(&authority, &topology, &joining).expect("active Registry");
    published.revision += 1;
    published.fleet_subnet_roots[0].status = FleetSubnetRootStatus::Draining;
    published.services = vec![active_pool_service()];
    validation::validate(&authority, &topology, &published)
        .expect("published Registry with one canonical service");
    let directory = directory_for_root(&authority, &topology, &published, principal(6))
        .expect("active Fleet Directory");

    assert_eq!(
        directory.provenance.registry,
        FleetRegistryOps::version(&authority, &topology, &published).expect("Registry version")
    );
    assert_eq!(directory.provenance.source_fleet_subnet_root, principal(6));
    assert_eq!(
        directory
            .fleet_subnet_roots
            .iter()
            .map(|entry| {
                (
                    entry.placement_subnet,
                    entry.fleet_subnet_root,
                    entry.status,
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (subnet(5), principal(6), FleetSubnetRootStatus::Draining),
            (subnet(7), principal(8), FleetSubnetRootStatus::Active)
        ]
    );
    assert_eq!(directory.services.len(), 1);
    assert_configured_service_projection(&directory.services[0], &published.services[0]);

    std::assert_matches!(
        directory_for_root(&authority, &topology, &joining, principal(6)),
        Err(FleetRegistryOpsError::FleetDirectoryRequiresPublishedRoots)
    );
    std::assert_matches!(
        directory_for_root(&authority, &topology, &published, principal(9)),
        Err(FleetRegistryOpsError::FleetDirectorySourceMissing)
    );
    published.services.clear();
    published.fleet_subnet_roots[0].status = FleetSubnetRootStatus::Removed;
    std::assert_matches!(
        directory_for_root(&authority, &topology, &published, principal(6)),
        Err(FleetRegistryOpsError::FleetDirectorySourceMissing)
    );
    let surviving_directory = directory_for_root(&authority, &topology, &published, principal(8))
        .expect("surviving root Directory with Removed peer");
    assert_eq!(
        surviving_directory
            .fleet_subnet_roots
            .iter()
            .map(|entry| entry.status)
            .collect::<Vec<_>>(),
        vec![
            FleetSubnetRootStatus::Removed,
            FleetSubnetRootStatus::Active
        ]
    );
}

fn assert_configured_service_projection(
    directory: &FleetDirectoryService,
    registry: &FleetServiceBinding,
) {
    // Intentionally exhaustive: Fleet discovery is configured topology, not
    // runtime health, readiness, replication or load-balancer evidence.
    let FleetDirectoryService {
        service,
        role,
        component_spec,
        mode,
        placement,
        members,
    } = directory;
    assert_eq!(service, &registry.service);
    assert_eq!(role, &registry.role);
    assert_eq!(component_spec, &registry.component_spec);
    assert_eq!(mode, &registry.mode);
    assert_eq!(placement, &registry.placement);
    assert_eq!(members.len(), registry.members.len());
    for (directory_member, registry_member) in members.iter().zip(&registry.members) {
        let FleetDirectoryServiceComponent {
            member_purpose,
            component,
            fleet_subnet_root,
            canister_id,
            group_placement,
            member_path,
        } = directory_member;
        assert_eq!(member_purpose, &registry_member.member_purpose);
        assert_eq!(component, &registry_member.component);
        assert_eq!(fleet_subnet_root, &registry_member.fleet_subnet_root);
        assert_eq!(canister_id, &registry_member.canister_id);
        assert_eq!(group_placement, &registry_member.group_placement);
        assert_eq!(member_path, &registry_member.member_path);
    }
}

#[test]
fn activation_rejects_empty_mixed_or_exhausted_registry_state() {
    let topology = topology();
    let authority = authority();
    let empty = validation::compile_genesis(&AppId::from("demo"), authority.clone(), &topology)
        .expect("valid genesis Registry");
    assert!(FleetRegistryOps::compile_active(&authority, &topology, &empty).is_err());

    let mut mixed = empty;
    mixed.fleet_subnet_roots = vec![
        root(&topology, 5, 6, &[("alpha", 1)]),
        root(&topology, 7, 8, &[("beta", 1)]),
    ];
    mixed.fleet_subnet_roots[1].status = FleetSubnetRootStatus::Active;
    assert!(FleetRegistryOps::compile_active(&authority, &topology, &mixed).is_err());

    let mut exhausted = mixed;
    exhausted.revision = u64::MAX;
    exhausted
        .fleet_subnet_roots
        .iter_mut()
        .for_each(|entry| entry.status = FleetSubnetRootStatus::Joining);
    assert!(FleetRegistryOps::compile_active(&authority, &topology, &exhausted).is_err());
}

#[test]
fn joining_compile_is_canonical_exact_idempotent_and_monotonic() {
    let topology = topology();
    let genesis = validation::compile_genesis(&AppId::from("demo"), authority(), &topology)
        .expect("valid genesis Registry");
    let later_subnet = root(&topology, 7, 8, &[("beta", 1)]);
    let first = compile_joining(
        &genesis.authority,
        &topology,
        &genesis,
        later_subnet.clone(),
    )
    .expect("first root joins");
    assert_eq!(first.revision, 2);

    let repeated = compile_joining(&first.authority, &topology, &first, later_subnet.clone())
        .expect("exact root retry");
    assert_eq!(repeated, first);

    let earlier_subnet = root(&topology, 5, 6, &[("alpha", 1)]);
    let second = compile_joining(&first.authority, &topology, &first, earlier_subnet.clone())
        .expect("second root joins");
    assert_eq!(second.revision, 3);
    assert_eq!(
        second
            .fleet_subnet_roots
            .iter()
            .map(|entry| entry.placement_subnet)
            .collect::<Vec<_>>(),
        vec![
            earlier_subnet.placement_subnet,
            later_subnet.placement_subnet
        ]
    );
}

#[test]
fn joining_compile_rejects_wrong_status_identity_conflicts_and_revision_exhaustion() {
    let topology = topology();
    let genesis = validation::compile_genesis(&AppId::from("demo"), authority(), &topology)
        .expect("valid genesis Registry");
    let joining = root(&topology, 5, 6, &[("alpha", 1)]);

    let mut active = joining.clone();
    active.status = FleetSubnetRootStatus::Active;
    std::assert_matches!(
        compile_joining(&genesis.authority, &topology, &genesis, active),
        Err(FleetRegistryOpsError::FleetSubnetRootJoinRequiresJoining)
    );

    let joined = compile_joining(&genesis.authority, &topology, &genesis, joining.clone())
        .expect("root joins");
    let same_subnet = root(&topology, 5, 7, &[("beta", 1)]);
    std::assert_matches!(
        compile_joining(&joined.authority, &topology, &joined, same_subnet),
        Err(FleetRegistryOpsError::FleetSubnetRootJoinIdentityConflict)
    );
    let same_principal = root(&topology, 7, 6, &[("beta", 1)]);
    std::assert_matches!(
        compile_joining(&joined.authority, &topology, &joined, same_principal),
        Err(FleetRegistryOpsError::FleetSubnetRootJoinIdentityConflict)
    );

    let mut exhausted = genesis;
    exhausted.revision = u64::MAX;
    std::assert_matches!(
        compile_joining(&exhausted.authority, &topology, &exhausted, joining),
        Err(FleetRegistryOpsError::RevisionExhausted)
    );
}

#[test]
fn registry_rejects_duplicate_root_principal_and_coordinator_collision() {
    let topology = topology();
    let mut duplicate = validation::compile_genesis(&AppId::from("demo"), authority(), &topology)
        .expect("valid genesis Registry");
    duplicate.fleet_subnet_roots = vec![
        root(&topology, 5, 8, &[("alpha", 1)]),
        root(&topology, 7, 8, &[("beta", 1)]),
    ];
    std::assert_matches!(
        validation::validate(&duplicate.authority, &topology, &duplicate),
        Err(FleetRegistryOpsError::DuplicateFleetSubnetRoot { .. })
    );

    let mut collision = validation::compile_genesis(&AppId::from("demo"), authority(), &topology)
        .expect("valid genesis Registry");
    collision.fleet_subnet_roots = vec![root(&topology, 5, 3, &[("alpha", 1)])];
    std::assert_matches!(
        validation::validate(&collision.authority, &topology, &collision),
        Err(FleetRegistryOpsError::RootPrincipalConflictsWithCoordinator)
    );
}

#[test]
fn genesis_requires_epoch_one_and_registry_authorities_remain_positive() {
    let topology = topology();
    let mut wrong_genesis = authority();
    wrong_genesis.epoch = 2;
    std::assert_matches!(
        validation::compile_genesis(&AppId::from("demo"), wrong_genesis, &topology),
        Err(FleetRegistryOpsError::GenesisAuthorityEpoch(2))
    );
    std::assert_matches!(
        validation::compile_genesis(&AppId::from("other"), authority(), &topology),
        Err(FleetRegistryOpsError::GenesisAppMismatch { .. })
    );

    let mut registry = validation::compile_genesis(&AppId::from("demo"), authority(), &topology)
        .expect("valid genesis Registry");
    registry.authority.epoch = 0;
    std::assert_matches!(
        validation::validate(&registry.authority, &topology, &registry),
        Err(FleetRegistryOpsError::NonPositiveAuthorityEpoch)
    );
}

#[test]
fn registry_roots_share_one_active_release_build() {
    let topology = topology();
    let mut registry = validation::compile_genesis(&AppId::from("demo"), authority(), &topology)
        .expect("valid genesis Registry");
    let first = root(&topology, 5, 6, &[("alpha", 1)]);
    let mut second = root(&topology, 7, 8, &[("beta", 1)]);
    second.active_release_set = release_set_for_build(10, 8);
    registry.fleet_subnet_roots = vec![first, second];

    std::assert_matches!(
        validation::validate(&registry.authority, &topology, &registry),
        Err(FleetRegistryOpsError::RootReleaseBuildMismatch { .. })
    );
}

#[test]
fn registry_authority_must_match_the_protected_expected_authority() {
    let topology = topology();
    let mut registry = validation::compile_genesis(&AppId::from("demo"), authority(), &topology)
        .expect("valid genesis Registry");
    let expected_authority = registry.authority.clone();
    registry.authority.binding.coordinator = principal(4);

    std::assert_matches!(
        validation::validate(&expected_authority, &topology, &registry),
        Err(FleetRegistryOpsError::AuthorityMismatch)
    );
}
