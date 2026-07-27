//! Module: ops::fleet_registry::tests
//!
//! Responsibility: freeze canonical Fleet Registry genesis and root-row invariants.
//! Does not own: persistence, synchronization, or lifecycle-effect coverage.
//! Boundary: exercises validation and encoding through one exact compiled Component Topology.

use super::*;
use crate::{
    bootstrap::parse_config_model,
    dto::fleet_registry::{FleetSubnetRootEntry, FleetSubnetRootStatus},
    ids::{
        AppId, CanonicalNetworkId, ComponentSpecAdmission, CyclesFundingBudget, FleetBinding,
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
                    canonical_network_id: CanonicalNetworkId::public_ic(),
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
        maximum_managed_canisters: 20,
        maximum_registry_bytes: 2_097_152,
        maximum_wasm_store_bytes: 40_000_000,
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

#[test]
fn genesis_is_revision_one_with_complete_specs_and_no_roots() {
    let topology = topology();
    let registry = validation::compile_genesis(&AppId::from("demo"), authority(), &topology)
        .expect("valid genesis Registry");

    assert_eq!(registry.revision, 1);
    assert_eq!(registry.component_specs.len(), 2);
    assert!(registry.fleet_subnet_roots.is_empty());
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
        "89c9969bf4cd41b7ddff132642b375a0070b4fd79c9a7cdf6be25701f3b7a73a"
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
